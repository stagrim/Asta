<script lang="ts">
	import { toast } from 'svelte-sonner';
	import * as Resizable from '$lib/components/ui/resizable';
	import type { TreeDirectory } from '$lib/server/sasta_client';
	import AppSidebar from './AppSiderbar.svelte';
	import { createFileManager } from './file-manager.svelte';
	import FileExplorer from './FileExplorer.svelte';
	import PreviewPanel from './PreviewPanel.svelte';
	import type { FileManagerAPI } from './types';
	import { getFiles, uploadFile } from './files.remote';
	import * as FileDropZone from '$lib/components/ui-extra/file-drop-zone';
	import { Button } from '$lib/components/ui-extra/button';
	import { XIcon } from '@lucide/svelte';

	const MAX_UPLOAD_SIZE = 50_000_000;

	let { api, fileTree }: { api: FileManagerAPI; fileTree: TreeDirectory } = $props();

	const fm = createFileManager(api, fileTree);

	const { directory, files } = uploadFile.fields;
	let loading = $state(false);
	let selectedFiles = $state<File[]>([]);
	let selectedFilesSize = $derived(selectedFiles.reduce((p, c) => p + c.size, 0));
	let fileInput = $state<HTMLInputElement | null>(null);

	// Sync svelte state with form field
	$effect(() => {
		if (fileInput) {
			const dt = new DataTransfer();
			selectedFiles.forEach((f) => dt.items.add(f));
			// Immutable property, so a DataTransfer object is used
			fileInput.files = dt.files;
		}
	});

	const onUpload: FileDropZone.FileDropZoneRootProps['onUpload'] = async (uploadedFiles) => {
		// Filter out files that are already in the selectedFiles
		// array from uploadedFiles before appending
		const uniqueNewFiles = uploadedFiles.filter((newFile) =>
			selectedFiles.every((existingFile) => existingFile.name !== newFile.name)
		);

		selectedFiles = [...selectedFiles, ...uniqueNewFiles];
	};

	const onFileRejected: FileDropZone.FileDropZoneRootProps['onFileRejected'] = async ({
		reason,
		file
	}) => {
		toast.error(`${file.name} failed to upload!`, { description: reason });
	};

	function removeFile(index: number) {
		selectedFiles = selectedFiles.filter((_, i) => i !== index);
	}
</script>

<!-- Just for testing -->
<form
	enctype="multipart/form-data"
	class="flex w-full flex-col gap-2 p-6"
	{...uploadFile.enhance(async ({ form, submit }) => {
		loading = true;
		try {
			await submit().updates(getFiles());

			form.reset();
			selectedFiles = [];
			directory.set(fm.currentPath);

			toast.success('Your attachments were uploaded');
		} catch (error: any) {
			toast.error(error.body?.message || 'Upload failed');
			await getFiles().refresh();
		}
		await fm.refresh();
		loading = false;
	})}
>
	<input {...directory.as('text')} hidden value={fm.currentPath} />

	<input {...files.as('file multiple')} bind:this={fileInput} class="hidden" />

	<FileDropZone.Root
		{onUpload}
		{onFileRejected}
		maxFileSize={MAX_UPLOAD_SIZE}
		fileCount={selectedFiles.length}
	>
		<FileDropZone.Trigger />
	</FileDropZone.Root>

	<div class="flex flex-col gap-2">
		{#each selectedFiles as file, i (file.name)}
			<div class="flex place-items-center justify-between gap-2">
				<div class="flex flex-col">
					<span>{file.name}</span>
					<span class="text-muted-foreground text-xs">
						{FileDropZone.displaySize(file.size)}
					</span>
				</div>
				<Button variant="outline" size="icon" type="button" onclick={() => removeFile(i)}>
					<XIcon />
				</Button>
			</div>
		{/each}
	</div>

	<div class="flex gap-4 items-center">
		<Button
			type="submit"
			class="w-fit"
			{loading}
			disabled={selectedFiles.length === 0 || selectedFilesSize > MAX_UPLOAD_SIZE}
		>
			Submit
		</Button>

		<Button
			type="button"
			variant="outline"
			onclick={() => {
				selectedFiles = [];
				directory.set(fm.currentPath);
			}}
		>
			Reset
		</Button>

		<span
			class="text-muted-foreground text-xs"
			class:text-red-400={selectedFilesSize > MAX_UPLOAD_SIZE}
		>
			{FileDropZone.displaySize(selectedFilesSize)} / {FileDropZone.displaySize(MAX_UPLOAD_SIZE)}
		</span>
	</div>
</form>

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
