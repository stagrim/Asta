<script lang="ts">
	import SearchIcon from '@lucide/svelte/icons/search';
	import HardDriveIcon from '@lucide/svelte/icons/hard-drive';
	import * as TreeView from '$lib/components/ui/tree-view';
	import * as Sidebar from '$lib/components/ui/sidebar';
	import { Label } from '$lib/components/ui/label';
	import type { TreeDirectory } from '$lib/api_bindings/files/TreeDirectory';
	import { History, House } from '@lucide/svelte';

	let {
		fileTree,
		selectedFolder,
		onFolderSelect
	}: {
		fileTree: TreeDirectory;
		selectedFolder: string;
		onFolderSelect: (folder: string) => void;
	} = $props();

	let searchQuery = $state('');
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
		<Sidebar.Group class="py-0">
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
							class="gap-3 {selectedFolder === ''
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
							class="gap-3 {selectedFolder === 'Recent'
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
					<TreeView.Root>
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
	<TreeView.Folder name={node.name}>
		{#each node.directories as child}
			{@render recursiveNode(child)}
		{/each}
	</TreeView.Folder>
{/snippet}
