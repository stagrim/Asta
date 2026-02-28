import { query, command, form } from '$app/server';
import { addFiles, deleteFiles, getAllPathsTree } from '$lib/server/sasta_client';
import { vDeleteFilesData, vFileUpload } from '$lib/server/sasta_client/valibot.gen';

export const getFiles = query(async () => {
	const res = await getAllPathsTree();
	if (res.error || !res.data) throw new Error('Could not load files');
	return res.data;
});

export const uploadFile = form(vFileUpload, async (form) => {
	const res = await addFiles({ body: {} });
	if (res.error) throw new Error('Failed to upload files');
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
