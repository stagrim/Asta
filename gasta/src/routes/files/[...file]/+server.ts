import { env } from '$env/dynamic/private';
import type { RequestHandler } from '@sveltejs/kit';

export const GET: RequestHandler = async ({ params, fetch }) => {
	const { file } = params;
	const response = await fetch(`${env.SERVER_URL}/files/${file}`);
	return new Response(response.body, {
		status: response.status,
		statusText: response.statusText,
		headers: response.headers
	});
};
