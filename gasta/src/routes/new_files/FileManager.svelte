<script lang="ts">
	import type { TreeDirectory } from '$lib/api_bindings/files/TreeDirectory';
	import type { TreeFile } from '$lib/api_bindings/files/TreeFile';
	import * as Sidebar from '$lib/components/ui/sidebar';
	import AppSidebar from './AppSiderbar.svelte';
	import FileExplorer from './FileExplorer.svelte';
	import PreviewPanel from './PreviewPanel.svelte';

	let { fileTree }: { fileTree: TreeDirectory } = $props();

	let selectedFolder = $state('');
	let selectedItem = $state<TreeFile | TreeDirectory | null>(null);
	let showPreview = $state(true);
	let viewMode = $state<'grid' | 'list'>('grid');

	const currentFiles = $derived(fileTree.files || []);
	const currentDirectories = $derived(fileTree.directories || []);

	function handleFolderSelect(folder: string) {
		selectedFolder = folder;
		selectedItem = null;
	}

	function handleFileSelect(item: TreeFile | TreeDirectory | null) {
		selectedItem = item;
		if (!showPreview) {
			showPreview = true;
		}
	}

	function togglePreview() {
		showPreview = !showPreview;
	}

	function setViewMode(mode: 'grid' | 'list') {
		viewMode = mode;
	}
</script>

<div
	class="group/filemanager relative flex w-full border rounded-xl overflow-hidden isolation-isolate bg-background"
>
	<Sidebar.Provider style="--sidebar-width-icon: 0px;">
		<AppSidebar {fileTree} {selectedFolder} onFolderSelect={handleFolderSelect} />

		<Sidebar.Inset>
			<main class="flex flex-1 overflow-hidden h-screen w-full">
				<FileExplorer
					files={currentFiles}
					directories={currentDirectories}
					{selectedItem}
					{selectedFolder}
					{viewMode}
					{showPreview}
					onFileSelect={handleFileSelect}
					onTogglePreview={togglePreview}
					onViewModeChange={setViewMode}
				/>

				{#if showPreview}
					<PreviewPanel file={selectedItem} onClose={togglePreview} />
				{/if}
			</main>
		</Sidebar.Inset>
	</Sidebar.Provider>
</div>
