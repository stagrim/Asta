<script lang="ts">
	import type { TreeDirectory } from '$lib/api_bindings/files/TreeDirectory';
	import type { TreeFile } from '$lib/api_bindings/files/TreeFile';
	import * as Sidebar from '$lib/components/ui/sidebar';
	import AppSidebar from './AppSiderbar.svelte';
	import FileExplorer from './FileExplorer.svelte';
	import PreviewPanel from './PreviewPanel.svelte';

	let { fileTree }: { fileTree: TreeDirectory } = $props();

	let currentPath = $state('/');
	let selectedItem = $state<TreeFile | TreeDirectory | null>(null);
	let showPreview = $state(true);
	let viewMode = $state<'grid' | 'list'>('grid');

	let currentFiles = $state<TreeFile[]>([]);
	let currentDirectories = $state<TreeDirectory[]>([]);

	$effect(() => {
		currentFiles = fileTree.files;
		currentDirectories = fileTree.directories;
	});

	function handleFolderSelect(directory: TreeDirectory | string) {
		if (typeof directory === 'string') {
			const dir = traverseTree(directory);
			if (dir) {
				currentPath = dir.id;
				selectedItem = null;

				currentFiles = dir.files;
				currentDirectories = dir.directories;
			} else {
				console.error(`${directory} was not found`);
			}
		} else {
			currentPath = directory.id;
			selectedItem = null;

			currentFiles = directory.files;
			currentDirectories = directory.directories;
		}
	}

	function traverseTree(path: string): TreeDirectory | null {
		const dirs = path.split('/').filter((s) => s);

		if (dirs.length === 0) {
			return fileTree;
		}

		let dir = fileTree;
		for (const x of dirs) {
			const res = dir.directories.find((d) => d.name === x);

			if (res) {
				dir = res;
			} else {
				return null;
			}
		}

		return dir;
	}

	function handleFileSelect(item: TreeFile | TreeDirectory | null) {
		selectedItem = item;
		if (!showPreview) {
			// showPreview = true;
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
		<AppSidebar {fileTree} {currentPath} onFolderSelect={handleFolderSelect} />

		<Sidebar.Inset>
			<main class="flex flex-1 overflow-hidden h-screen w-full">
				<!-- TODO: Only give the current directory item instead of passing the containing directories and files separately -->
				<FileExplorer
					files={currentFiles}
					directories={currentDirectories}
					{selectedItem}
					selectedFolder={currentPath}
					{viewMode}
					{showPreview}
					onFileSelect={handleFileSelect}
					onFolderSelect={handleFolderSelect}
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
