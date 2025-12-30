<script lang="ts">
	import { Label } from '$lib/components/ui/label';
	import type { TreeDirectory } from '$lib/api_bindings/files/TreeDirectory';
	import { FolderPlus, HardDrive, History, House, Search, Upload } from '@lucide/svelte';
	import * as TreeView from '$lib/components/ui/tree-view';
	import * as Resizable from '$lib/components/ui/resizable';
	import { ScrollArea } from '$lib/components/ui/scroll-area';
	import { Button } from '$lib/components/ui/button';
	import * as Sheet from '$lib/components/ui/sheet';
	import * as InputGroup from '$lib/components/ui/input-group';
	import * as ButtonGroup from '$lib/components/ui/button-group';
	import { useFileManager } from './file-manager.svelte';
	import { watch } from 'runed';

	// svelte-ignore non_reactive_update
	let pane: ReturnType<typeof Resizable.Pane>;

	const fm = useFileManager();

	let searchQuery = $state('');

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

	// function handleDragOver(e: DragEvent) {
	// 	e.preventDefault(); // Essential to allow dropping
	// 	e.stopPropagation();
	// 	isDragOver = true;
	// }
</script>

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
						<InputGroup.Input type="search" placeholder="Search..." bind:value={searchQuery} />
						<InputGroup.Addon>
							<Search />
						</InputGroup.Addon>
					</InputGroup.Root>
				</div>

				<ButtonGroup.Root class="flex w-full py-4">
					<Button class="grow" variant="secondary" size="sm">
						<Upload /> Upload
					</Button>
					<ButtonGroup.Separator />
					<Button class="grow" variant="secondary" size="sm"><FolderPlus /> New Folder</Button>
				</ButtonGroup.Root>

				<div class="mt-4">
					<h4 class="my-2 rounded-md px-4 text-xs text-muted-foreground">Quick Access</h4>
					<div class="grid gap-1">
						<Button
							variant={fm.currentPath === '/' ? 'secondary' : 'ghost'}
							class="w-full justify-start h-8"
							onclick={() => fm.navigate('/')}
						>
							<House class="h-4 w-4" />
							Home
						</Button>
						<Button
							variant={fm.currentPath === 'Recent' ? 'secondary' : 'ghost'}
							class="w-full justify-start h-8"
							onclick={() => fm.navigate('Recent')}
						>
							<History class="h-4 w-4" />
							Recent
						</Button>
					</div>
				</div>

				<div class="mt-4">
					<h4 class="mb-1 rounded-md px-4 text-xs text-muted-foreground">Filesystem</h4>
					<div class="grid gap-1">
						<TreeView.Root selectedId={fm.currentPath}>
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
		minSize={15}
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
