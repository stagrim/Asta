<script lang="ts">
	import { toast } from 'svelte-sonner';
	import {
		createFolder,
		getFiles,
		moveItem,
		removeFile,
		renameItem,
		uploadFile
	} from './files.remote';
	import type { FileManagerAPI } from '$lib/components/file-manager/types';
	import FileManager from '$lib/components/file-manager/FileManager.svelte';
	import type { FormContext } from '$lib/components/file-manager/AppSiderbar.svelte';

	const backendProvider: FileManagerAPI = {
		getFileTree: () => getFiles(),
		createFile: async () => false,
		createFolder: (directory) => createFolder({ directory, files: [] }),
		// TODO: Confirm deletion and show playlists using this file
		deleteFile: (ids) => removeFile({ body: { ids } }),
		renameItem: (id_from, id_to) => renameItem({ body: { ids_from: [id_from], ids_to: [id_to] } }),
		moveItems: (ids_from, id_to) => moveItem({ ids_from, id_to })
	};

	const { directory, files } = uploadFile.fields;
</script>

{#snippet fileUploadSnippet({
	InternalUI,
	bindFileInput,
	setLoading,
	resetFiles,
	currentPath,
	refreshManager
}: FormContext)}
	<form
		enctype="multipart/form-data"
		class="flex w-full flex-col gap-2 p-6"
		{...uploadFile.enhance(async ({ element, submit }) => {
			setLoading(true);
			try {
				await submit().updates(getFiles());

				element.reset();
				resetFiles();
				directory.set(currentPath);

				toast.success('Your attachments were uploaded');
			} catch (error: any) {
				toast.error(error.body?.message || 'Upload failed');
				await getFiles().refresh();
			}
			await refreshManager();
			setLoading(false);
		})}
	>
		<input {...directory.as('text')} hidden value={currentPath} />
		<input {...files.as('file multiple')} class="hidden" use:bindFileInput />

		{@render InternalUI()}
	</form>
{/snippet}

<FileManager
	api={backendProvider}
	fileTree={await backendProvider.getFileTree()}
	uploadFormSnippet={fileUploadSnippet}
/>
