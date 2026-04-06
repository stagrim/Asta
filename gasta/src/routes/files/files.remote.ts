import { query, command, form } from '$app/server';
import * as v from 'valibot';
import {
	addFiles,
	deleteFiles,
	getAllPathsTree,
	renameFiles,
	type TreeDirectory
} from '$lib/server/sasta_client';
import {
	vDeleteFilesData,
	vFileUpload as generatedFileUpload,
	vRenameFilesData
} from '$lib/server/sasta_client/valibot.gen';
import { error } from '@sveltejs/kit';

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
	// Who knew file uploads could be so irritating??
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

	await getFiles().refresh();

	if (res.error) {
		console.error('Upload failed:', res.error);
		if (res.error.type === 'Error') {
			console.error(res.error.content?.message);
			error(res.response.status, `Failed to upload files: ${res.error.content?.message}`);
		} else {
			error(res.response.status, 'Could not upload file');
		}
	}

	return true;
});

export const createFolder = command(vFileUpload, async (body) => {
	const root = await getFiles();
	const dir = traverseTree(body.directory, root);
	if (dir) {
		error(403, 'Folder already exists');
	}

	const res = await addFiles({ body });
	if (res.error) {
		console.error(res.error);
		throw new Error('Failed to create dir');
	}
	await getFiles().refresh();
	return true;
});

export const removeFile = command(vDeleteFilesData, async ({ body }) => {
	const res = await deleteFiles({ body });
	if (res.error) {
		console.error(res.error);
		throw new Error('Failed to delete files');
	}
	await getFiles().refresh();
	return true;
});

export const renameItem = command(vRenameFilesData, async ({ body }) => {
	if (body.ids_from.length != 1 || body.ids_to.length != 1) {
		error(403, 'Only one item may be renamed at a time');
	}
	const res = await renameFiles({ body });
	if (res.error) {
		console.error(res.error);
		if (res.error.type === 'Error') {
			error(500, res.error.content?.message);
		} else {
			error(500, 'Could not rename file');
		}
	}
	await getFiles().refresh();
	return true;
});

/** ids_from may contain multiple paths to be moved.
 *  ids_to may only contain one directory where all should be moved.
 */
// The server rename is very flexible, so must have some logic to mold a into a pure file/dir 'move' function
export const moveItem = command(
	v.object({
		ids_from: v.array(v.string()),
		id_to: v.string()
	}),
	async ({ ids_from, id_to }) => {
		if (ids_from.length === 0) {
			error(403, 'Must give at least one path to move');
		}
		const ids_to = ids_from.map((p) => `${id_to}${p.split('/').at(-1)}`);
		const res = await renameFiles({ body: { ids_from, ids_to } });
		if (res.error) {
			console.error(res.error);
			if (res.error.type === 'Error') {
				error(500, res.error.content?.message);
			} else {
				error(500, 'Could not rename file');
			}
		}
		await getFiles().refresh();
		return true;
	}
);

function traverseTree(path: string, root: TreeDirectory): TreeDirectory | null {
	const dirs = path.split('/').filter((s) => s);

	if (dirs.length === 0) {
		return root;
	}

	let dir = root;
	for (const x of dirs) {
		const res = dir.directories.find((d) => d.name === x);
		if (res) {
			dir = res;
		} else {
			return null;
		}
	}
	return dir;
}
