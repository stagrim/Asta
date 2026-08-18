<script lang="ts" module>
	export function previews(file: TreeFile) {
		const authorized_extensions = ['jpg', 'jpeg', 'png', 'webp', 'gif', 'svg'];
		// TODO: break out into a function
		const ext = file.name.split('.').at(-1)!;
		return authorized_extensions.includes(ext) ? `/files/${file.id}` : false;
	}
</script>

<script lang="ts">
	import FileIcon from './FileIcon.svelte';
	import { Button } from '../ui/button';
	import { Separator } from '../ui/separator';
	import DownloadIcon from '@lucide/svelte/icons/download';
	import PencilIcon from '@lucide/svelte/icons/pencil';
	import Trash2Icon from '@lucide/svelte/icons/trash-2';
	import EyeIcon from '@lucide/svelte/icons/eye';
	import { filesize } from 'filesize';
	import { CheckIcon, CopyIcon, ExternalLink, Files, Folder } from '@lucide/svelte';
	import * as Resizable from '../ui/resizable';
	import { PressedKeys, watch } from 'runed';
	import * as Sheet from '../ui/sheet';
	import { useFileManager } from './file-manager.svelte';
	import type { TreeDirectory, TreeFile } from '$lib/server/sasta_client';
	import { toast } from 'svelte-sonner';
	import * as AlertDialog from '../ui/alert-dialog';
	import { Input } from '../ui/input';
	import * as TreeView from '../tree-view';
	import * as InputGroup from '../ui/input-group';

	// svelte-ignore non_reactive_update
	let pane: ReturnType<typeof Resizable.Pane>;
	const keys = new PressedKeys();

	const fm = useFileManager();

	watch(
		() => fm.previewOpen,
		() => {
			if (pane) {
				if (fm.previewOpen && pane.isCollapsed()) {
					pane.expand();
				} else if (!fm.previewOpen && pane.isExpanded()) {
					pane.collapse();
				}
			}
		}
	);

	let renameName = $state('');
	let renameExtension = $state('');
	let renamePaneOpen = $state(false);
	let renameError = $state('');

	const openRenamePane = () => {
		const selected = fm.oneSelected();
		if (selected) {
			const [name, ext] = selected.name.split('.');
			renameName = name;
			renameExtension = ext;
			renamePaneOpen = true;
			renameError = '';
		}
	};

	let moveValue = $state('');
	let movePaneOpen = $state(false);
	let moveError = $state('');

	const openMovePane = () => {
		moveValue = '';
		movePaneOpen = true;
		moveError = '';
	};

	watch(
		() => keys.all,
		() => {
			if (keys.has('f2')) {
				openRenamePane();
			}
		}
	);

	let copied = $state(false);
</script>

{#snippet previewContent()}
	{@const selectedItem = fm.oneSelected()}
	<aside class="w-full h-full shrink-0 border-l border-border bg-card flex flex-col">
		<header class="flex items-center justify-between px-4 py-3 border-b border-border">
			<h2 class="font-medium text-foreground">Preview</h2>
		</header>

		{#if selectedItem}
			<div class="flex-1 overflow-auto">
				<div class="p-4">
					<div class="flex flex-col items-center mb-6">
						<div class="rounded-lg mb-3">
							<!-- TODO: Make this a util function? -->
							<!-- Check if a TreeDirectory -->
							{#if 'directories' in selectedItem}
								<Folder class="w-12 h-12" />
							{:else}
								{@const previewURL = previews(selectedItem)}
								{#if previewURL}
									<img class="rounded-lg" src={previewURL} alt="" />
								{:else}
									<FileIcon extension={selectedItem.name.split('.').at(-1)} size="lg" />
								{/if}
							{/if}
						</div>
						<h3 class="text-sm font-medium text-foreground text-center break-all">
							{selectedItem.name}
						</h3>
					</div>

					<Separator class="my-4" />

					<div class="space-y-4">
						{#if 'size' in selectedItem}
							{@const astaURL = `ASTA:/${selectedItem.id}`}
							<div>
								<h4 class="text-xs font-medium text-muted-foreground uppercase tracking-wider mb-2">
									Details
								</h4>
								<dl class="space-y-2">
									<div class="flex justify-between">
										<dt class="text-sm text-muted-foreground">Size</dt>
										<dd class="text-sm text-foreground">{filesize(selectedItem.size)}</dd>
									</div>
									<div class="flex justify-between">
										<dt class="text-sm text-muted-foreground">Modified</dt>
										<dd class="text-sm text-foreground">
											{new Date(selectedItem.date).toLocaleString()}
										</dd>
									</div>
								</dl>
							</div>

							<Separator class="my-4" />
							<div>
								<h4 class="text-xs font-medium text-muted-foreground uppercase tracking-wider mb-2">
									Asta URL
								</h4>
								<InputGroup.Root>
									<InputGroup.Input value={astaURL} class="text-zinc-500" readonly />
									<InputGroup.Addon align="inline-end">
										<InputGroup.Button
											aria-label="Copy"
											title="Copy"
											size="icon-xs"
											onclick={async () => {
												try {
													await navigator.clipboard.writeText(astaURL);
													copied = true;
													setTimeout(() => (copied = false), 2000);
												} catch (err) {
													toast('Could not copy Uuid, ' + err);
												}
											}}
										>
											{#if copied}
												<CheckIcon />
											{:else}
												<CopyIcon />
											{/if}
										</InputGroup.Button>
									</InputGroup.Addon>
								</InputGroup.Root>
							</div>
						{:else}
							<div>
								<h4 class="text-xs font-medium text-muted-foreground uppercase tracking-wider mb-2">
									Details
								</h4>
								<dl class="space-y-2">
									<div class="flex justify-between">
										<dt class="text-sm text-muted-foreground">Size</dt>
										<dd class="text-sm text-foreground">
											{selectedItem.directories.length + selectedItem.files.length} item(s)
										</dd>
									</div>
								</dl>
							</div>
						{/if}

						<Separator />

						<div class="@container">
							<h4 class="text-xs font-medium text-muted-foreground uppercase tracking-wider mb-2">
								Actions
							</h4>
							<div class="grid gap-2 grid-cols-1 @[230px]:grid-cols-2">
								{#if 'size' in selectedItem}
									<Button
										variant="secondary"
										size="sm"
										class="gap-2 col-span-full"
										onclick={() =>
											window
												.open(`/files${selectedItem.id}`, '_blank', 'noopener,noreferrer')
												?.focus()}
									>
										<ExternalLink class="w-4 h-4" />
										Open
									</Button>
									<a href="/files{selectedItem.id}" download>
										<Button variant="secondary" size="sm" class="gap-2 w-full">
											<DownloadIcon class="w-4 h-4" />
											Download
										</Button>
									</a>
								{/if}
								<Button
									variant="secondary"
									size="sm"
									class="gap-2"
									onclick={() => openRenamePane()}
								>
									<PencilIcon class="w-4 h-4" />
									Rename
								</Button>
								<AlertDialog.Root bind:open={renamePaneOpen}>
									<AlertDialog.Content>
										<AlertDialog.Header>
											<AlertDialog.Title>Rename {selectedItem.name}</AlertDialog.Title>
										</AlertDialog.Header>
										<div class="flex flex-col">
											<div class="flex gap-4 items-center text-muted-foreground">
												<PencilIcon size={50} />
												<Input bind:value={renameName} placeholder="New name" autofocus />
												{#if renameExtension}
													<p>.{renameExtension}</p>
												{/if}
											</div>
											{#if renameError}
												<span class="text-red-400">{renameError}</span>
											{/if}
										</div>
										<AlertDialog.Footer>
											<AlertDialog.Cancel type="button">Cancel</AlertDialog.Cancel>
											<AlertDialog.Action
												disabled={!renameName ||
													`${renameName}.${renameExtension}` === selectedItem.name ||
													renameName === selectedItem.name}
												onclick={async () => {
													console.info(renameName);

													renameError = await fm.renameItem(
														selectedItem,
														`${renameName}${renameExtension ? '.' + renameExtension : '/'}`
													);
													if (!renameError) {
														renamePaneOpen = false;
													}
												}}>Rename</AlertDialog.Action
											>
										</AlertDialog.Footer>
									</AlertDialog.Content>
								</AlertDialog.Root>
								<Button variant="secondary" size="sm" class="gap-2" onclick={() => openMovePane()}>
									<Folder class="w-4 h-4" />
									Move
								</Button>
								<Button
									variant="destructive"
									size="sm"
									class="gap-2 {'directories' in selectedItem && 'col-span-full'}"
									onclick={() => fm.deleteFile([selectedItem]) || toast('Could not delete file')}
								>
									<Trash2Icon class="w-4 h-4" />
									Delete
								</Button>
							</div>
						</div>
					</div>
				</div>
			</div>
		{:else if fm.nbrSelected() > 1}
			<div class="flex flex-col items-center justify-center text-muted-foreground p-4 gap-2">
				<Files class="w-12 h-12 mb-3 stroke-1" />
				<p class="text-sm text-center">{fm.nbrSelected()} items selected</p>

				<Separator class="my-2" />

				<div class="@container w-full">
					<h4 class="text-xs font-medium text-muted-foreground uppercase tracking-wider mb-2">
						Actions
					</h4>
					<div class="grid gap-2 grid-cols-1 @[230px]:grid-cols-2">
						<Button variant="secondary" size="sm" class="gap-2" onclick={() => openMovePane()}>
							<Folder class="w-4 h-4" />
							Move
						</Button>
						<Button
							variant="destructive"
							size="sm"
							class="gap-2"
							onclick={() => fm.deleteFile(fm.getSelected()) || toast('Could not delete files')}
						>
							<Trash2Icon class="w-4 h-4" />
							Delete
						</Button>
					</div>
				</div>
			</div>
		{:else}
			<div class="flex-1 flex flex-col items-center justify-center text-muted-foreground p-4">
				<EyeIcon class="w-12 h-12 mb-3 stroke-1" />
				<p class="text-sm text-center">Select a file to preview</p>
			</div>
		{/if}
	</aside>
{/snippet}

<AlertDialog.Root bind:open={movePaneOpen}>
	<AlertDialog.Content>
		<AlertDialog.Header>
			<AlertDialog.Title>Move item(s)</AlertDialog.Title>
		</AlertDialog.Header>
		<TreeView.Root selectedId={moveValue}>
			{@render recursiveNode(fm.root)}
		</TreeView.Root>
		{#if moveError}
			<span class="text-red-400">{moveError}</span>
		{/if}
		<AlertDialog.Footer>
			<AlertDialog.Cancel type="button">Cancel</AlertDialog.Cancel>
			<AlertDialog.Action
				disabled={!moveValue}
				onclick={async () => {
					moveError = await fm.moveItems(fm.getSelected(), moveValue);
					if (!moveError) {
						movePaneOpen = false;
					}
				}}>Move</AlertDialog.Action
			>
		</AlertDialog.Footer>
	</AlertDialog.Content>
</AlertDialog.Root>

{#if fm.isMobile}
	<Sheet.Root bind:open={fm.previewOpen}>
		<Sheet.Content side="right" class="p-0 w-75">
			{@render previewContent()}
		</Sheet.Content>
	</Sheet.Root>
{:else}
	<Resizable.Pane
		bind:this={pane}
		collapsible={true}
		onCollapse={() => (fm.previewOpen = false)}
		onExpand={() => (fm.previewOpen = true)}
		defaultSize={0}
		class={{ 'max-w-[40%] min-w-45': fm.previewOpen }}
		maxSize={40}
	>
		{#if fm.previewOpen}
			{@render previewContent()}
		{/if}
	</Resizable.Pane>
{/if}

{#snippet recursiveNode(node: TreeDirectory)}
	{#if node.directories.length}
		<TreeView.Folder
			open={false}
			name={node.name}
			id={node.id}
			onclick={() => (moveValue = node.id)}
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
			onclick={() => (moveValue = node.id)}
		/>
	{/if}
{/snippet}
