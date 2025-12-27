<script lang="ts">
	import FileManager from './FileManager.svelte';

	let { data } = $props();
</script>

<FileManager fileTree={data.fileTree} />
