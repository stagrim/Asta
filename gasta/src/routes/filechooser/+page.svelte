<script lang="ts">
	import { FileButton } from '@skeletonlabs/skeleton';
	import { Filemanager } from 'wx-svelte-filemanager';
	import type { PageData } from './$types';
	import { enhance } from '$app/forms';
	import AstaTheme from '../files/AstaTheme.svelte';

	let { data }: { data: PageData } = $props();

	const authorized_extensions = ['.jpg', '.jpeg', '.png', '.webp'];

	const file_data = [
		{
			id: '/Code',
			size: 4096,
			date: new Date(2023, 11, 2, 17, 25),
			type: 'folder'
		},
		{
			id: '/Music',
			size: 4096,
			date: new Date(2023, 11, 1, 14, 45),
			type: 'folder'
		},

		{
			id: '/Info.txt',
			size: 1000,
			date: new Date(2023, 10, 30, 6, 13),
			type: 'file'
		},
		{
			//parent: "/Code/Datepicker",
			id: '/Code/Datepicker/Year.svelte',
			size: 1595,
			date: new Date(2023, 11, 7, 15, 23),
			type: 'file'
		},
		{
			id: '/Pictures/162822515312968813.png',
			size: 510885,
			date: new Date(2023, 11, 1, 14, 45),
			type: 'file'
		}
	];

	function init(api: any) {
		api.on('open-file', ({ id }: { id: string }) => {
			alert(`File ${id} is double-clicked`);
		});
	}
</script>

<!-- <form method="POST" use:enhance enctype="multipart/form-data">
	<FileButton name="file" />
	<button class="btn" type="submit">Upload</button>
	<input class="input" type="text" name="directory" value="/hello" />
</form>
<br />
<br />
<br />
{#each data.payload.content as c}
	{c}
{/each} -->
{JSON.stringify(data.payload)}

<AstaTheme>
	<Filemanager data={file_data} readonly={true} {init} />
</AstaTheme>
