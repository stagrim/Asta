<script lang="ts" module>
	import type { Snippet } from 'svelte';
	import type { Action } from 'svelte/action';

	export type FormContext = {
		InternalUI: Snippet;
		bindFileInput: Action<HTMLInputElement>;
		setLoading: (state: boolean) => void;
		resetFiles: () => void;
		currentPath: string;
		refreshManager: () => Promise<void>;
	};

	function debounce<T extends (...args: any[]) => void>(fn: T, ms: number) {
		let timer: ReturnType<typeof setTimeout>;
		return (...args: Parameters<T>) => {
			clearTimeout(timer);
			timer = setTimeout(() => fn(...args), ms);
		};
	}
</script>

<script lang="ts">
	import { Label } from '$lib/components/ui/label';
	import { Folder, FolderPlus, HardDrive, History, House, Search, Upload } from '@lucide/svelte';
	import * as TreeView from '$lib/components/tree-view';
	import * as Resizable from '$lib/components/ui/resizable';
	import { ScrollArea } from '$lib/components/ui/scroll-area';
	import { Button, buttonVariants } from '$lib/components/ui/button';
	import * as Sheet from '$lib/components/ui/sheet';
	import * as InputGroup from '$lib/components/ui/input-group';
	import * as ButtonGroup from '$lib/components/ui/button-group';
	import { useFileManager } from './file-manager.svelte';
	import { watch } from 'runed';
	import type { TreeDirectory } from '$lib/server/sasta_client';
	import * as AlertDialog from '$lib/components/ui/alert-dialog';
	import * as FileDropZone from '$lib/components/ui-extra/file-drop-zone';
	import { Button as ButtonExtra } from '$lib/components/ui-extra/button';
	import { XIcon } from '@lucide/svelte';
	import { toast } from 'svelte-sonner';
	import { Separator } from '$lib/components/ui/separator';
	import { Input } from '$lib/components/ui/input';
	import { CurrentPathType } from './types';

	let { uploadFormSnippet }: { uploadFormSnippet?: Snippet<[FormContext]> } = $props();

	// svelte-ignore non_reactive_update
	let pane: ReturnType<typeof Resizable.Pane>;

	const fm = useFileManager();

	let localSearchQuery = $state(fm.searchQuery);

	let lastPath = $state<
		| {
				type: CurrentPathType.Recent;
		  }
		| {
				type: CurrentPathType.Path;
				path: string;
		  }
	>();
	const executeSearch = debounce((query: string) => {
		if (query) {
			if (fm.currentPath.type !== CurrentPathType.Search) {
				lastPath = fm.currentPath;
			}
			fm.navigateSearch(query);
		} else if (fm.currentPath.type === CurrentPathType.Search) {
			if (lastPath) {
				lastPath.type === CurrentPathType.Path ? fm.navigate(lastPath.path) : fm.navigateRecent();
			} else {
				fm.navigate(fm.root);
			}
		}
	}, 250);

	// Sync search field with url parameters
	watch(
		() => fm.currentPath.type,
		() => {
			if (fm.currentPath.type !== CurrentPathType.Search) {
				localSearchQuery = '';
			} else {
				localSearchQuery = fm.searchQuery;
			}
		}
	);

	watch(
		() => fm.sidebarOpen,
		() => {
			if (pane) {
				if (fm.sidebarOpen && pane.isCollapsed()) {
					pane.expand();
				} else if (!fm.sidebarOpen && pane.isExpanded()) {
					pane.collapse();
				}
			}
		}
	);

	const MAX_UPLOAD_SIZE = 50_000_000;

	let loading = $state(false);
	let selectedFiles = $state<File[]>([]);
	let selectedFilesSize = $derived(selectedFiles.reduce((p, c) => p + c.size, 0));
	let fileInput = $state<HTMLInputElement | null>(null);

	// Action exposed to the parent to bind the hidden input
	const bindFileInput: Action<HTMLInputElement> = (node) => {
		fileInput = node;
		return {
			destroy() {
				if (fileInput === node) fileInput = null;
			}
		};
	};

	function setLoading(state: boolean) {
		loading = state;
	}
	function resetFiles() {
		selectedFiles = [];
	}

	// Sync svelte state with form field
	$effect(() => {
		if (fileInput) {
			const dt = new DataTransfer();
			selectedFiles.forEach((f) => dt.items.add(f));
			// Immutable property, so a DataTransfer object is used
			fileInput.files = dt.files;
		}
	});

	const onUpload: FileDropZone.FileDropZoneRootProps['onUpload'] = async (uploadedFiles) => {
		// Filter out files that are already in the selectedFiles
		// array from uploadedFiles before appending
		const uniqueNewFiles = uploadedFiles.filter((newFile) =>
			selectedFiles.every((existingFile) => existingFile.name !== newFile.name)
		);

		selectedFiles = [...selectedFiles, ...uniqueNewFiles];
	};

	const onFileRejected: FileDropZone.FileDropZoneRootProps['onFileRejected'] = async ({
		reason,
		file
	}) => {
		toast.error(`${file.name} failed to upload!`, { description: reason });
	};

	function removeFile(index: number) {
		selectedFiles = selectedFiles.filter((_, i) => i !== index);
	}

	let folderPaneOpen = $state(false);
	let folderName = $state('');
	let folderError = $state('');

	// function handleDragOver(e: DragEvent) {
	// 	e.preventDefault(); // Essential to allow dropping
	// 	e.stopPropagation();
	// 	isDragOver = true;
	// }
</script>

{#snippet InternalFileUploadUI()}
	<FileDropZone.Root
		{onUpload}
		{onFileRejected}
		maxFileSize={MAX_UPLOAD_SIZE}
		fileCount={selectedFiles.length}
	>
		<FileDropZone.Trigger />
	</FileDropZone.Root>

	<div class="flex flex-col gap-2">
		<ScrollArea class="max-h-[50vh]">
			<div class="p-4">
				{#each selectedFiles as file, i (file.name)}
					<div class="flex place-items-center justify-between gap-2">
						<div class="flex flex-col">
							<span>{file.name}</span>
							<span class="text-muted-foreground text-xs">
								{FileDropZone.displaySize(file.size)}
							</span>
						</div>
						<Button variant="outline" size="icon" type="button" onclick={() => removeFile(i)}>
							<XIcon />
						</Button>
					</div>
					<Separator class="my-2" />
				{/each}
			</div>
		</ScrollArea>
	</div>

	<div class="flex gap-2 items-center">
		<AlertDialog.Cancel type="button">Cancel</AlertDialog.Cancel>
		<Button
			class="mr-auto"
			type="button"
			variant="outline"
			onclick={resetFiles}
			disabled={selectedFiles.length <= 0}
		>
			Reset
		</Button>
		<span
			class="text-muted-foreground text-xs"
			class:text-red-400={selectedFilesSize > MAX_UPLOAD_SIZE}
		>
			{FileDropZone.displaySize(selectedFilesSize)} / {FileDropZone.displaySize(MAX_UPLOAD_SIZE)}
		</span>
		<ButtonExtra
			type="submit"
			class="w-fit"
			{loading}
			disabled={selectedFiles.length === 0 || selectedFilesSize > MAX_UPLOAD_SIZE}
		>
			Submit
		</ButtonExtra>
	</div>
{/snippet}

{#snippet sidebarContent()}
	<div class="flex h-full flex-col bg-sidebar border-r-0">
		<div class="flex items-center px-4 py-3 border-b">
			<div class="flex items-center gap-2">
				<HardDrive class="h-4 w-4 text-primary" />
				<span class="font-semibold">File Manager</span>
			</div>
		</div>

		<ScrollArea class="flex-1 pt-4 overflow-auto">
			<div class="px-2">
				<div class="relative py-1">
					<Label for="search" class="sr-only">Search</Label>
					<InputGroup.Root>
						<InputGroup.Input
							type="search"
							placeholder="Search..."
							bind:value={localSearchQuery}
							oninput={() => executeSearch(localSearchQuery)}
						/>
						<InputGroup.Addon>
							<Search />
						</InputGroup.Addon>
					</InputGroup.Root>
				</div>

				<ButtonGroup.Root class="flex w-full py-4">
					<AlertDialog.Root>
						<AlertDialog.Trigger
							class="grow {buttonVariants({ variant: 'secondary', size: 'sm' })}"
						>
							<Upload /> Upload
						</AlertDialog.Trigger>
						<AlertDialog.Content>
							<AlertDialog.Header>
								<AlertDialog.Title>Upload Files</AlertDialog.Title>
							</AlertDialog.Header>
							{#if uploadFormSnippet}
								{@render uploadFormSnippet({
									InternalUI: InternalFileUploadUI,
									bindFileInput,
									setLoading,
									resetFiles,
									currentPath:
										fm.currentPath.type === CurrentPathType.Path ? fm.currentPath.path : '/',
									refreshManager: async () => await fm.refresh()
								})}
							{/if}
						</AlertDialog.Content>
					</AlertDialog.Root>
					<ButtonGroup.Separator />
					<AlertDialog.Root bind:open={folderPaneOpen}>
						<AlertDialog.Trigger
							class="grow {buttonVariants({ variant: 'secondary', size: 'sm' })}"
						>
							<FolderPlus /> New Folder
						</AlertDialog.Trigger>
						<AlertDialog.Content>
							<AlertDialog.Header>
								<AlertDialog.Title>New Folder</AlertDialog.Title>
							</AlertDialog.Header>
							<div class="flex flex-col">
								<div class="flex gap-4 items-center text-muted-foreground">
									<Folder size={50} />
									<Input bind:value={folderName} placeholder="Folder name" />
								</div>
								{#if folderError}
									<span class="text-red-400">{folderError}</span>
								{/if}
							</div>
							<AlertDialog.Footer>
								<AlertDialog.Cancel type="button">Cancel</AlertDialog.Cancel>
								<AlertDialog.Action
									disabled={!folderName}
									onclick={async () => {
										folderError = await fm.createFolder(folderName);
										if (!folderError) {
											folderPaneOpen = false;
											//TODO: set selection to newly created folder
										}
									}}>Create</AlertDialog.Action
								>
							</AlertDialog.Footer>
						</AlertDialog.Content>
					</AlertDialog.Root>
				</ButtonGroup.Root>

				<div class="mt-4">
					<h4 class="my-2 rounded-md px-4 text-xs text-muted-foreground">Quick Access</h4>
					<div class="grid gap-1">
						<Button
							variant={fm.currentPath.type === CurrentPathType.Path && fm.currentPath.path === '/'
								? 'secondary'
								: 'ghost'}
							class="w-full justify-start h-8"
							onclick={() => fm.navigate('/')}
						>
							<House class="h-4 w-4" />
							Home
						</Button>
						<Button
							variant={fm.currentPath.type === CurrentPathType.Recent ? 'secondary' : 'ghost'}
							class="w-full justify-start h-8"
							onclick={() => fm.navigateRecent()}
						>
							<History class="h-4 w-4" />
							Recent
						</Button>
						<!-- TODO: Unused tab, where all files unused by Asta is put -->
					</div>
				</div>

				<div class="mt-4">
					<h4 class="mb-1 rounded-md px-4 text-xs text-muted-foreground">Filesystem</h4>
					<div class="grid gap-1">
						<TreeView.Root
							selectedId={fm.currentPath.type === CurrentPathType.Path ? fm.currentPath.path : ''}
						>
							{#each fm.root.directories ?? [] as child}
								{@render recursiveNode(child)}
							{/each}
						</TreeView.Root>
					</div>
				</div>
			</div>
		</ScrollArea>
	</div>
{/snippet}

{#if fm.isMobile}
	<Sheet.Root bind:open={fm.sidebarOpen}>
		<Sheet.Content side="left" class="p-0 w-[80vw]">
			{@render sidebarContent()}
		</Sheet.Content>
	</Sheet.Root>
{:else}
	<Resizable.Pane
		bind:this={pane}
		collapsible={true}
		onCollapse={() => (fm.sidebarOpen = false)}
		onExpand={() => (fm.sidebarOpen = true)}
		defaultSize={20}
		class={{ 'max-w-[40%] min-w-58': fm.sidebarOpen }}
		maxSize={40}
	>
		{#if fm.sidebarOpen}
			{@render sidebarContent()}
		{/if}
	</Resizable.Pane>
{/if}

{#snippet recursiveNode(node: TreeDirectory)}
	{#if node.directories.length}
		<TreeView.Folder
			open={false}
			name={node.name}
			id={node.id}
			ondblclick={() => fm.navigate(node)}
		>
			{#each node.directories as child}
				{@render recursiveNode(child)}
			{/each}
		</TreeView.Folder>
	{:else}
		<TreeView.Folder
			open={false}
			name={node.name}
			id={node.id}
			ondblclick={() => fm.navigate(node)}
		/>
	{/if}
{/snippet}
