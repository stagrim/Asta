import type { PageServerLoad } from './$types';
import fs from 'fs';

export const load: PageServerLoad = async ({ locals }) => {
	let markdown = null;
	try {
		markdown = fs.readFileSync('Greet.md').toString();
	} catch {}
	return { name: locals.name, markdown };
};
