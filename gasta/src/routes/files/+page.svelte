<script lang="ts">
	import { Filemanager } from 'wx-svelte-filemanager';
	import type { PageData } from './$types';
	import { deserialize } from '$app/forms';
	import type { ActionResult } from '@sveltejs/kit';
	import { toastStore } from '$lib/stores';
	import AstaTheme from './AstaTheme.svelte';
	import { page } from '$app/state';

	let { data }: { data: PageData } = $props();

	const authorized_extensions = ['.jpg', '.jpeg', '.png', '.webp', '.gif', '.svg'];

	let api: any;

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

	function previewURL(
		file: {
			id: string;
			name: string;
			date: Date;
			type: 'file' | 'folder' | undefined;
			size: number;
			file: File;
			ext: string;
		},
		width: number,
		height: number
	) {
		const ext = file.ext;
		if (
			ext === 'png' ||
			ext === 'jpg' ||
			ext === 'jpeg' ||
			ext === 'webp' ||
			ext === 'gif' ||
			ext === 'svg'
		)
			return `/files/${file.id}`;

		return false;
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

		api.on('set-path', ({ id }: { id: string; selected?: []; panel?: 0 | 1 }) => {
			console.log({
				t: page.url.searchParams.get('p') ?? '/',
				i: id,
				res: (page.url.searchParams.get('p') ?? '/') !== id
			});
			if ((page.url.searchParams.get('p') ?? '/') !== id) {
				console.log('PUshed');
				history.pushState({ p: id }, '', `?p=${id}`);
				page.url.searchParams.set('p', id);
			}
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
		previews={previewURL}
		bind:this={api}
	/>
</AstaTheme>
