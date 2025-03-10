import { env } from '$env/dynamic/private';
import { fail } from '@sveltejs/kit';
import type { Actions, PageServerLoad } from './$types';
import type { Payload } from '$lib/api_bindings/files/Payload';
import type { DeleteFilesRequest } from '$lib/api_bindings/files/DeleteFilesRequest';

export const load: PageServerLoad = async () => {
	const payload: Payload = await fetch(`${env.SERVER_URL}/api/files`).then((d) => d.json());

	if (payload.type == 'Error') {
		console.error(`Error: ${payload}`);
		throw Error();
	}

	console.log(payload);

	return { payload };
};

export const actions = {
	create: async ({ request }) => {
		const formData = await request.formData();

		// if (!(formData.file as File).name || (formData.file as File).name === 'undefined') {
		// 	return fail(400, {
		// 		error: true,
		// 		message: 'You must provide a file to upload'
		// 	});
		// }

		// const { file } = formData as { file: File };
		console.log(formData);

		const test = await fetch(`${env.SERVER_URL}/api/files`, {
			method: 'POST',
			body: formData
		});
		if (test.ok) {
			return await test.json();
		} else {
			return fail(test.status, await test.json());
		}
	},
	delete: async ({ request }) => {
		const formData = await request.formData();
		console.log(formData);
		const body: DeleteFilesRequest = { ids: formData.getAll('ids').map((id) => id.toString()) };
		const test = await fetch(`${env.SERVER_URL}/api/files`, {
			method: 'DELETE',
			headers: {
				'Content-Type': 'application/json'
			},
			body: JSON.stringify(body)
		});
		if (test.ok) {
			return await test.json();
		} else {
			return fail(test.status, await test.json());
		}
	}
} satisfies Actions;
