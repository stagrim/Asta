<script lang="ts">
	import FileManager from './FileManager.svelte';
	import { getFiles, removeFile, renameFile, uploadFile } from './files.remote';
	import type { FileManagerAPI } from './types';

	let { data } = $props();

	const backendProvider: FileManagerAPI = {
		getFileTree: () => getFiles(),
		createFile: async () => false,
		deleteFile: (uuid) => removeFile({ body: { ids: [uuid] } }),
		renameFile: (uuid, newName) => renameFile()
	};
	let fileTree = $derived(await backendProvider.getFileTree());

	const {directory, files} = uploadFile.fields
</script>

<!-- Just for testing -->
<form {...uploadFile} enctype="multipart/form-data">
	<input {...directory.as('text')} value="/" />
	<input {...files.as('file multiple')} />
	<button>submit</button>
</form>
<FileManager {fileTree} />
