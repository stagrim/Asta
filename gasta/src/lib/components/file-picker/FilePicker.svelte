<script lang="ts">
	import type { TreeDirectory, TreeFile } from '#lib/server/sasta_client/index.ts';
	import { AlertDialog as AlertDialogPrimitive } from 'bits-ui';
	import * as AlertDialog from '../ui/alert-dialog';
	import type { Snippet } from 'svelte';
	import { type FileManagerAPI, type ValidFileTypeFilters } from '../file-manager/types';
	import FileManager from '../file-manager/FileManager.svelte';
	import type { FormContext } from '../file-manager/AppSiderbar.svelte';

	export type { ValidFileTypeFilters };

	let {
		api,
		onSelected,
		defaultFilterFileType = $bindable<ValidFileTypeFilters>('all'),
		uploadFormSnippet,
		children
	}: {
		api: FileManagerAPI;
		onSelected: (selected: TreeFile) => void;
		defaultFilterFileType?: ValidFileTypeFilters;
		uploadFormSnippet?: Snippet<[FormContext]>;
		children: Snippet;
	} = $props();

	let selectedItems = $state<(TreeFile | TreeDirectory)[]>([]);
	let selectedFile = $state<TreeFile | undefined>();
	let open = $state(false);
	let selectedFileTypeValue: ValidFileTypeFilters = $state(defaultFilterFileType);
</script>

<AlertDialog.Root
	bind:open
	onOpenChange={(state) => {
		if (state) {
			selectedItems = [];
			selectedFile = undefined;
			selectedFileTypeValue = defaultFilterFileType;
		}
	}}
>
	<AlertDialog.Trigger type="button">{@render children()}</AlertDialog.Trigger>

	<AlertDialog.Portal>
		<AlertDialog.Overlay />
		<AlertDialogPrimitive.Content
			data-slot="file-picker-content"
			class="data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 fixed left-[50%] top-[50%] z-50 w-11/12 max-w-[90vw] h-[90vh] translate-x-[-50%] translate-y-[-50%] duration-200 outline-none"
		>
			<FileManager
				class="h-full shadow-lg"
				{api}
				fileTree={await api.getFileTree()}
				{uploadFormSnippet}
				bind:selectedItems
				bind:selectedFile
				bind:filterFileType={selectedFileTypeValue}
			>
				{#snippet footerSnippet()}
					<p class="text-sm text-muted-foreground truncate">
						{#if selectedItems.length > 1}
							{selectedItems.length} items selected — select a single file to continue
						{:else if selectedItems.length === 1 && !selectedFile}
							Folder selected — select a single file to continue
						{:else if selectedFile}
							Selected: <span class="font-medium text-foreground">{selectedFile.name}</span>
						{:else}
							Select a file
						{/if}
					</p>
					<div class="flex items-center gap-2 shrink-0">
						<AlertDialog.Cancel type="button">Close</AlertDialog.Cancel>
						<AlertDialog.Action
							type="button"
							disabled={!selectedFile || selectedItems.length !== 1}
							onclick={() => {
								if (selectedFile && selectedItems.length === 1) {
									open = false;
									onSelected(selectedFile);
								}
							}}
						>
							Select
						</AlertDialog.Action>
					</div>
				{/snippet}
			</FileManager>
		</AlertDialogPrimitive.Content>
	</AlertDialog.Portal>
</AlertDialog.Root>
