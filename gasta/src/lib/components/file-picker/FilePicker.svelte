<script lang="ts">
	import type { TreeDirectory, TreeFile } from '$lib/server/sasta_client';
	import * as AlertDialog from '../ui/alert-dialog';
	import * as TreeView from '../tree-view';
	import { previews } from '../file-manager/PreviewPanel.svelte';
	import FileIcon from '../file-manager/FileIcon.svelte';
	import { ScrollArea } from '../ui/scroll-area';
	import type { Snippet } from 'svelte';

	let {
		root,
		onSelected,
		children
	}: { root: TreeDirectory; onSelected: (selected: TreeFile) => void; children: Snippet } =
		$props();

	let selectedId = $state('');
	let selectedFile = $state<TreeFile | undefined>();

	let open = $state(false);
</script>

<AlertDialog.Root
	bind:open
	onOpenChange={(state) => {
		if (state) {
			selectedId = '';
			selectedFile = undefined;
		}
	}}
>
	<AlertDialog.Trigger type="button">{@render children()}</AlertDialog.Trigger>

	<AlertDialog.Content class="w-11/12 max-w-[90vw] h-[90vh] flex flex-col sm:max-w-full">
		<AlertDialog.Header class="shrink-0 pb-4">
			<AlertDialog.Title>Choose a file</AlertDialog.Title>
		</AlertDialog.Header>

		<div class="flex-1 min-h-0 overflow-hidden">
			<div class="flex h-full gap-4 flex-col lg:flex-row">
				<ScrollArea class="min-h-0 pr-2 flex-1">
					<TreeView.Root bind:selectedId bind:complementryData={selectedFile}>
						{#each root.directories ?? [] as child}
							{@render recursiveNode(child)}
						{/each}
						{#each root.files ?? [] as child}
							<TreeView.Item label={child.name} value={child.id} complementryData={child} />
						{/each}
					</TreeView.Root>
				</ScrollArea>

				<div class="min-h-0 flex justify-center items-center max-h-full lg:w-1/4 max-lg:max-h-1/4">
					{#if selectedFile}
						{@const previewURL = previews(selectedFile)}
						{#if previewURL}
							<img class="rounded-lg object-contain h-full" src={previewURL} alt="" />
						{:else}
							<FileIcon extension={selectedFile.name.split('.').at(-1)} size="xl" />
						{/if}
					{:else}
						<p class="text-muted-foreground mt-10 wrap-break-word text-center">
							Select a file to preview
						</p>
					{/if}
				</div>
			</div>
		</div>

		<AlertDialog.Footer class="shrink-0 pb-4">
			<AlertDialog.Cancel>Close</AlertDialog.Cancel>
			<AlertDialog.Action
				disabled={!selectedFile}
				onclick={() => {
					if (selectedFile) {
						open = false;
						onSelected(selectedFile);
					}
				}}
			>
				Select
			</AlertDialog.Action>
		</AlertDialog.Footer>
	</AlertDialog.Content>
</AlertDialog.Root>

{#snippet recursiveNode(node: TreeDirectory)}
	{#if node.directories.length}
		<TreeView.Folder open={false} name={node.name} id={node.id}>
			{#each node.directories as child}
				{@render recursiveNode(child)}
			{/each}
			{#each node.files as child}
				<TreeView.Item label={child.name} value={child.id} complementryData={child} />
			{/each}
		</TreeView.Folder>
	{:else}
		<TreeView.Folder open={false} name={node.name} id={node.id} />
	{/if}
{/snippet}
