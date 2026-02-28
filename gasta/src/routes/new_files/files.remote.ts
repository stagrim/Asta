import { query, command, form } from '$app/server';
import * as v from 'valibot';
import { addFiles, deleteFiles, getAllPathsTree } from '$lib/server/sasta_client';
import {
	vDeleteFilesData,
	vFileUpload as generatedFileUpload
} from '$lib/server/sasta_client/valibot.gen';
import { createClient } from '$lib/server/sasta_client/client';
import { createClientConfig } from '$lib/server/sasta-api';
import { client as globalClient } from '$lib/server/sasta_client/client.gen';

// Overwrite file
const vFileUpload = v.object({
	...generatedFileUpload.entries,
	files: v.array(v.file())
});

export const getFiles = query(async () => {
	const res = await getAllPathsTree();
	if (res.error || !res.data) throw new Error('Could not load files');
	return res.data;
});

export const uploadFile = form(vFileUpload, async (body) => {
	// Perform some rituals to keep deno happy and make file upload work
	// Who knew file uploads are so irritating??
	// Hours wasted: 3
	const formData = new FormData();

	formData.append('directory', body.directory);

	for (const file of body.files) {
		const buffer = await file.arrayBuffer();

		const nativeFile = new File([buffer], file.name, {
			type: file.type
		});
		formData.append('files', nativeFile);
	}
	const res = await addFiles({
		body: {} as any,

		fetch: async (url, init) => {
			return fetch(url, {
				...init,
				headers: {},
				body: formData
			});
		}
	});

	if (res.error) {
		console.error('Upload failed:', res.error);
		throw new Error('Failed to upload files');
	}
	return true;
});

export const removeFile = command(vDeleteFilesData, async ({ body }) => {
	const res = await deleteFiles({ body });
	if (res.error) throw new Error('Failed to delete files');
	return true;
});

export const renameFile = command(async () => {
	console.log('TODO: implement');
	// const res = await { path: { uuid: payload.uuid }, body: { name: payload.name } };
	// if (res.error) throw new Error('Failed to rename');
	return true;
});
