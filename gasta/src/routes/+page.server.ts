import { redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';
import fs from 'node:fs';

export const load: PageServerLoad = async ({ locals }) => {
	const session = await locals.auth();
	
	if (!session?.user) {
		redirect(302, "/login")
	}

	let markdown = null;
	try {
		markdown = fs.readFileSync('Greet.md').toString();
	} catch {
		console.log('No Greet.md file found');
	}
	return { name: session?.user.name, markdown };
};
