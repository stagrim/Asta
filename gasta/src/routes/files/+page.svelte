<script lang="ts">
	// @ts-ignore
	import { Filemanager, getMenuOptions } from 'wx-svelte-filemanager';
	import type { PageData } from './$types';
	import { deserialize } from '$app/forms';
	import type { ActionResult } from '@sveltejs/kit';
	import { toastStore } from '$lib/stores';
	import AstaTheme from './AstaTheme.svelte';
	import { page } from '$app/state';

	let { data }: { data: PageData } = $props();
	let api: any;
	type Type = {
		date: Date;
		ext: string;
		id: string;
		name: string;
		parent: string;
		size: number;
		type: 'file' | 'folder';
	};

	if (!page.url.searchParams.has('p')) {
		history.replaceState({ p: '/' }, '', '?p=/');
	}
	// TODO: Why it no work on backwards to no query parameter??
	$effect(() => {
		if (page.url.href) {
			console.log(page.url.href);
		}
		if (api) {
			api.exec('set-path', {
				id: page.url.searchParams.get('p') ?? '/'
			});
		}
	});

	function previews(file: Type, width: number, height: number) {
		const authorized_extensions = ['jpg', 'jpeg', 'png', 'webp', 'gif', 'svg'];
		const ext = file.ext;
		return authorized_extensions.includes(ext) ? `/files/${file.id}` : false;
	}

	function menuOptions(mode: 'file' | 'folder' | 'add' | 'body' | 'multiselect', item: Type) {
		switch (mode) {
			case 'file':
				return [
					{
						icon: 'wxi-external',
						text: 'Open in new Tab',
						hotkey: 'Ctrl+O',
						id: 'open',
						handler: ({ context }: { context: Type }) =>
							window.open(`/files${context.id}`, '_blank')
					},
					...getMenuOptions(mode)
				];
			case 'add':
				return [
					{
						icon: 'mdi mdi-file-plus-outline',
						text: 'Add new folder',
						id: 'add-folder'
					},
					{
						icon: 'mdi mdi-file-upload-outline',
						text: 'Upload file',
						id: 'upload',
						type: 'upload'
					}
				];
			default:
				return getMenuOptions(mode);
		}
	}

	function init(api: any): void | false {
		api.intercept(
			'create-file',
			async ({
				parent,
				file,
				newId
			}: {
				file: {
					name: string;
					date: Date;
					type: 'file' | 'folder' | undefined;
					size: number;
					file: File;
				};
				parent: string;
				newId?: string;
			}) => {
				const body = new FormData();
				if (file.type === 'folder') {
					body.append('directory', `${parent === '/' ? '' : parent}/${file.name}`);
				} else {
					body.append('file', file.file);
					body.append('directory', parent);
				}
				const response = await fetch('?/create', {
					method: 'POST',
					body
				});
				const result: ActionResult = deserialize(await response.text());

				if (result.type === 'failure') {
					$toastStore.trigger({
						message: result.data?.content.message,
						timeout: 9999,
						background: 'variant-filled-error',
						hideDismiss: false
					});
					return false;
				}
			}
		);

		api.on('delete-files', async ({ ids }: { ids: string[] }) => {
			const body = new FormData();
			ids.forEach((id) =>
				body.append(
					'ids',
					data.payload.content.find((v) => v.id == id)?.type === 'folder' ? id + '/' : id
				)
			);
			const response = await fetch('?/delete', {
				method: 'POST',
				body
			});

			const result: ActionResult = deserialize(await response.text());

			if (result.type === 'failure') {
				$toastStore.trigger({
					message: result.data?.content.message,
					timeout: 9999,
					background: 'variant-filled-error',
					hideDismiss: false
				});
				return false;
			}
		});

		api.on('download-file', ({ id }: { id: string }) => {
			const link = `/files/${encodeURIComponent(id)}`;
			const anchor = document.createElement('a');
			anchor.href = link;
			anchor.download = link;
			anchor.click();
		});

		api.on('set-path', ({ id }: { id: string; selected?: []; panel?: 0 | 1 }) => {
			if (id.startsWith('/')) {
				// console.log({
				// 	t: page.url.searchParams.get('p') ?? '/',
				// 	i: id,
				// 	res: (page.url.searchParams.get('p') ?? '/') !== id
				// });
				if ((page.url.searchParams.get('p') ?? '/') !== id) {
					history.pushState({ p: id }, '', `?p=${id}`);
					page.url.searchParams.set('p', id);
				}
			}
		});

		api.on('open-file', ({ id }: { id: string }) => {
			console.log('Double clicked ' + id);
		});
	}
	console.log(data.payload.content);
</script>

<!-- <form method="POST" use:enhance enctype="multipart/form-data">
	<FileButton name="file" />
	<button class="btn" type="submit">Upload</button>
	<input class="input" type="text" name="directory" value="/hello" />
</form> -->
<!--
<br />
<br />
<br />
{#each data.payload.content as c}
	{c}
{/each} -->

<AstaTheme>
	<Filemanager
		data={data.payload.content}
		{init}
		icons={'simple'}
		{previews}
		{menuOptions}
		bind:this={api}
	/>
</AstaTheme>
