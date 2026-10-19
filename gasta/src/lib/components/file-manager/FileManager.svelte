<script lang="ts">
	import * as Resizable from '#lib/components/ui/resizable/index.js';
	import type { TreeDirectory, TreeFile } from '#lib/server/sasta_client/index.ts';
	import AppSidebar from './AppSiderbar.svelte';
	import { createFileManager } from './file-manager.svelte';
	import FileExplorer from './FileExplorer.svelte';
	import PreviewPanel from './PreviewPanel.svelte';
	import type { FileManagerAPI, ValidFileTypeFilters } from './types';
	import type { Snippet } from 'svelte';
	import type { FormContext } from './AppSiderbar.svelte';
	import { cn } from '#lib/utils.js';
	import { watch } from 'runed';

	let {
		api,
		fileTree,
		uploadFormSnippet,
		footerSnippet,
		selectedItems = $bindable<(TreeFile | TreeDirectory)[]>([]),
		selectedFile = $bindable<TreeFile | undefined>(undefined),
		filterFileType = $bindable<ValidFileTypeFilters>('all'),
		title = 'File Manager',
		class: className
	}: {
		api: FileManagerAPI;
		fileTree: TreeDirectory;
		uploadFormSnippet?: Snippet<[FormContext]>;
		footerSnippet?: Snippet;
		selectedItems?: (TreeFile | TreeDirectory)[];
		selectedFile?: TreeFile | undefined;
		filterFileType?: ValidFileTypeFilters;
		title?: string;
		class?: string;
	} = $props();

	// svelte-ignore state_referenced_locally
	const fm = createFileManager(api, fileTree, title, filterFileType);

	// Deselect files not matching the current filter
	watch(
		() => filterFileType,
		() => {
			for (const item of fm.getSelected()) {
				if ('size' in item && !fm.matchesFilter(item)) {
					fm.removeSelected(item);
				}
			}
		}
	);
	filterFileType = fm.filterFileType;
	selectedItems = fm.getSelected();

	$effect(() => {
		const single = fm.oneSelected();
		selectedFile = single && 'size' in single ? single : undefined;
	});
</script>

<div
	class={cn(
		'group/filemanager relative flex flex-col w-full h-[calc(100dvh-6rem)] border rounded-xl overflow-hidden isolation-isolate bg-background',
		className
	)}
>
	<Resizable.PaneGroup direction="horizontal" class="flex-1 min-h-0">
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

	{#if footerSnippet}
		<div
			class="shrink-0 border-t border-border bg-card px-4 py-3 flex items-center justify-between gap-4"
		>
			{@render footerSnippet()}
		</div>
	{/if}
</div>
