import { env } from '$env/dynamic/private';
import { fail } from '@sveltejs/kit';
import type { Actions, PageServerLoad } from './$types';
import type { Payload } from '$lib/api_bindings/files/Payload';

export const load: PageServerLoad = async () => {
	const payload: Payload = await fetch(`${env.SERVER_URL}/api/files`).then((d) => d.json());

	if (payload.type == 'Error') {
		console.error(`Error: ${payload}`);
		throw Error();
	}

	console.log(payload);

	return { payload };
};
