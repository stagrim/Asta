<script lang="ts">
	import FileManager from './FileManager.svelte';

	let { data } = $props();
	console.log(data.fileTree);
</script>

<FileManager fileTree={data.fileTree} />
