<script lang="ts">
	import FileManager from './FileManager.svelte';
	import { getFiles, removeFile, renameFile } from './files.remote';
	import type { FileManagerAPI } from './types';

	let { data } = $props();

	const backendProvider: FileManagerAPI = {
		getFileTree: () => getFiles(),
		createFile: () => ,
		deleteFile: (uuid) => removeFile({ body: { ids: [uuid] } }),
		renameFile: (uuid, newName) => renameFile()
	};
	let fileTree = $derived(await backendProvider.getFileTree());
</script>

<!-- {JSON.stringify(await getFiles())} -->
<FileManager {fileTree} />
