import { env } from '$env/dynamic/private';

export const GET = async ({ params, fetch }) => {
	const { file } = params;
	const f = await fetch(`${env.SERVER_URL}/files/${file}`);
	return f;
};
