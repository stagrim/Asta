<script lang="ts">
	import SearchIcon from '@lucide/svelte/icons/search';
	import HardDriveIcon from '@lucide/svelte/icons/hard-drive';
	import * as Sidebar from '$lib/components/ui/sidebar';
	import { Label } from '$lib/components/ui/label';
	import type { TreeDirectory } from '$lib/api_bindings/files/TreeDirectory';
	import { History, House } from '@lucide/svelte';
	import * as TreeView from '$lib/components/ui/tree-view';

	let {
		fileTree,
		currentPath,
		onFolderSelect
	}: {
		fileTree: TreeDirectory;
		currentPath: string;
		onFolderSelect: (directory: string | TreeDirectory) => void;
	} = $props();

	let searchQuery = $state('');

	// function handleDragOver(e: DragEvent) {
	// 	e.preventDefault(); // Essential to allow dropping
	// 	e.stopPropagation();
	// 	isDragOver = true;
	// }
</script>

<Sidebar.Root
	collapsible="icon"
	class="border-r border-sidebar-border relative group-data-[collapsible=icon]:w-0! overflow-hidden"
>
	<Sidebar.Header class="p-4">
		<div class="flex items-center gap-2 px-2">
			<HardDriveIcon class="h-5 w-5 text-primary" />
			<span class="font-semibold text-sidebar-foreground group-data-[collapsible=icon]:hidden">
				File Manager
			</span>
		</div>
	</Sidebar.Header>

	<Sidebar.Content>
		<Sidebar.Group class="py-1">
			<Sidebar.GroupContent class="relative">
				<Label for="search" class="sr-only">Search</Label>
				<Sidebar.Input id="search" placeholder="search" class="pl-8" bind:value={searchQuery} />
				<SearchIcon
					class="pointer-events-none absolute left-2 top-1/2 size-4 -translate-y-1/2 select-none opacity-50"
				/>
			</Sidebar.GroupContent>
		</Sidebar.Group>

		<Sidebar.Group>
			<Sidebar.GroupLabel class="text-xs font-medium text-muted-foreground px-4">
				Quick Access
			</Sidebar.GroupLabel>
			<Sidebar.GroupContent>
				<Sidebar.Menu>
					<Sidebar.MenuItem>
						<Sidebar.MenuButton
							onclick={() => onFolderSelect('')}
							class="gap-3 {currentPath === '/'
								? 'bg-sidebar-accent text-sidebar-accent-foreground'
								: ''}"
						>
							<House class="h-4 w-4" />
							<span>Home</span>
						</Sidebar.MenuButton>
					</Sidebar.MenuItem>
					<Sidebar.MenuItem>
						<Sidebar.MenuButton
							onclick={() => onFolderSelect('Recent')}
							class="gap-3 {currentPath === 'Recent'
								? 'bg-sidebar-accent text-sidebar-accent-foreground'
								: ''}"
						>
							<History class="h-4 w-4" />
							<span>Recent</span>
						</Sidebar.MenuButton>
					</Sidebar.MenuItem>
				</Sidebar.Menu>
			</Sidebar.GroupContent>
		</Sidebar.Group>

		<Sidebar.Group>
			<Sidebar.GroupLabel class="text-xs font-medium text-muted-foreground px-4">
				Folders
			</Sidebar.GroupLabel>
			<Sidebar.GroupContent>
				<Sidebar.Menu>
					<TreeView.Root bind:selectedId={currentPath}>
						{#each fileTree.directories ?? [] as child}
							{@render recursiveNode(child)}
						{/each}
					</TreeView.Root>
				</Sidebar.Menu>
			</Sidebar.GroupContent>
		</Sidebar.Group>
	</Sidebar.Content>
</Sidebar.Root>

{#snippet recursiveNode(node: TreeDirectory)}
	{#if node.directories.length}
		<TreeView.Folder open={false} name={node.name} id={node.id}>
			{#each node.directories as child}
				{@render recursiveNode(child)}
			{/each}
		</TreeView.Folder>
	{:else}
		<TreeView.Folder open={false} name={node.name} id={node.id} />
	{/if}
{/snippet}
