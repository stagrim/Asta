import type { PageServerLoad } from './$types';
import fs from 'node:fs';

export const load: PageServerLoad = ({ locals }) => {
	let markdown = null;
	try {
		markdown = fs.readFileSync('Greet.md').toString();
	} catch {
		console.log('No Greet.md file found');
	}
	return { name: locals.name, markdown };
};
