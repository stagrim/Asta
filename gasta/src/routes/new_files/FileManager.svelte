<script lang="ts">
	import type { TreeDirectory } from '$lib/api_bindings/files/TreeDirectory';
	import type { TreeFile } from '$lib/api_bindings/files/TreeFile';
	import * as Resizable from '$lib/components/ui/resizable';
	import { IsMobile } from '$lib/hooks/is-mobile.svelte';
	import AppSidebar from './AppSiderbar.svelte';
	import FileExplorer from './FileExplorer.svelte';
	import PreviewPanel from './PreviewPanel.svelte';

	const isMobile = new IsMobile();

	let { fileTree }: { fileTree: TreeDirectory } = $props();

	let currentPath = $state('/');
	let selectedItem = $state<TreeFile | TreeDirectory | null>(null);
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
	}

	function setViewMode(mode: 'grid' | 'list') {
		viewMode = mode;
	}

	let appSideBarOpen = $state(!isMobile.current);
	let previewPanelOpen = $state(false);
</script>

<div
	class="group/filemanager relative flex w-full h-full border rounded-xl overflow-hidden isolation-isolate bg-background"
>
	<Resizable.PaneGroup direction="horizontal">
		<AppSidebar
			{fileTree}
			{currentPath}
			onFolderSelect={handleFolderSelect}
			bind:open={appSideBarOpen}
		/>

		<Resizable.Handle />

		<Resizable.Pane>
			<main class="flex flex-1 overflow-hidden h-full w-full">
				<!-- TODO: Only give the current directory item instead of passing the containing directories and files separately -->

				<FileExplorer
					files={currentFiles}
					directories={currentDirectories}
					{selectedItem}
					selectedFolder={currentPath}
					{viewMode}
					onFileSelect={handleFileSelect}
					onFolderSelect={handleFolderSelect}
					onViewModeChange={setViewMode}
					bind:appSideBarOpen
					bind:previewPanelOpen
				/>
			</main>
		</Resizable.Pane>

		<Resizable.Handle />

		<PreviewPanel file={selectedItem} bind:open={previewPanelOpen} />
	</Resizable.PaneGroup>
</div>
