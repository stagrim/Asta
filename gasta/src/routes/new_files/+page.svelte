<script lang="ts">
	import FileManager from './FileManager.svelte';
	import { getFiles, removeFile, renameFile } from './files.remote';
	import type { FileManagerAPI } from './types';

	let { data } = $props();

	const backendProvider: FileManagerAPI = {
		getFileTree: () => getFiles(),
		createFile: async () => false,
		// TODO: Confirm deletion and show playlists using this file
		deleteFile: (ids) => removeFile({ body: { ids } }),
		renameFile: (uuid, newName) => renameFile()
	};
</script>

<FileManager api={backendProvider} fileTree={await backendProvider.getFileTree()} />
