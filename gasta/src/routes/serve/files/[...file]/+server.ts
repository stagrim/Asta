import { promises as fs } from 'fs';
import path from 'path';

export async function GET({ params }) {
	const { file } = params;
	const normalized_file = path.normalize(file).replace(/^(\.\.(\/|\\|$))+/, '');
	const filePath = path.join('./files/', normalized_file);

	try {
		const data = await fs.readFile(filePath);
		// Determine the Content-Type from the file extension
		// const contentType = 'determine the content type here';

		return new Response(data, { status: 200 });
	} catch {
		// Handle errors, like file not found
		return new Response('File not found', { status: 404 });
	}
}
