<script lang="ts">
	import { toast } from 'svelte-sonner';
	import * as Resizable from '$lib/components/ui/resizable';
	import type { TreeDirectory } from '$lib/server/sasta_client';
	import AppSidebar from './AppSiderbar.svelte';
	import { createFileManager } from './file-manager.svelte';
	import FileExplorer from './FileExplorer.svelte';
	import PreviewPanel from './PreviewPanel.svelte';
	import type { FileManagerAPI } from './types';

	let { api, fileTree }: { api: FileManagerAPI; fileTree: TreeDirectory } = $props();

	const fm = createFileManager(api, fileTree);
</script>

<div
	class="group/filemanager relative flex w-full h-full border rounded-xl overflow-hidden isolation-isolate bg-background"
>
	<Resizable.PaneGroup direction="horizontal">
		<AppSidebar />

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
