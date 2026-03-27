<script lang="ts">
	import * as Resizable from '$lib/components/ui/resizable';
	import type { TreeDirectory } from '$lib/server/sasta_client';
	import AppSidebar from './AppSiderbar.svelte';
	import { createFileManager } from './file-manager.svelte';
	import FileExplorer from './FileExplorer.svelte';
	import PreviewPanel from './PreviewPanel.svelte';
	import type { FileManagerAPI } from './types';
	import type { Snippet } from 'svelte';
	import type { FormContext } from './AppSiderbar.svelte';

	let {
		api,
		fileTree,
		uploadFormSnippet
	}: { api: FileManagerAPI; fileTree: TreeDirectory; uploadFormSnippet?: Snippet<[FormContext]> } =
		$props();

	const fm = createFileManager(api, fileTree);
</script>

<div
	class="group/filemanager relative flex w-full h-[calc(100dvh-6rem)] border rounded-xl overflow-hidden isolation-isolate bg-background"
>
	<Resizable.PaneGroup direction="horizontal">
		<AppSidebar {uploadFormSnippet} />

		<Resizable.Handle />

		<Resizable.Pane class="w-full">
			<main class="flex overflow-hidden h-full w-full">
				<FileExplorer />
			</main>
		</Resizable.Pane>

		<Resizable.Handle />

		<PreviewPanel />
	</Resizable.PaneGroup>
</div>
