import { defineConfig } from '@hey-api/openapi-ts';
import 'dotenv/config';

//@ts-ignore
const SERVER_URL = process.env.SERVER_URL;

export default defineConfig({
	input: `${SERVER_URL}/api-docs/openapi.json`,
	output: './src/lib/server/sasta_client',
	plugins: [
		{
			name: '@hey-api/client-fetch',
			runtimeConfigPath: '../sasta-api'
		},
		'valibot',
		{
			name: '@hey-api/sdk'
		}
	]
});
