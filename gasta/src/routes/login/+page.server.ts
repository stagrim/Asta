import { redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';
import { isInDevEnvironment } from '$lib/utils';

export const load: PageServerLoad = async (event) => {
	const session = await event.locals.auth();

	if (session?.user) {
		redirect(302, '/');
	}

	return {
		isInDevEnvironment
	};
};
