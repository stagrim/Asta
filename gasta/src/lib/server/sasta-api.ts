import type { CreateClientConfig } from './sasta_client/client.gen';

const SERVER_URL = process.env.SERVER_URL;

export const createClientConfig: CreateClientConfig = (config) => ({
	...config,
	baseUrl: SERVER_URL
});
