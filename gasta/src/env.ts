import { defineEnvVars } from '@sveltejs/kit/env';
import { building } from '$app/env';
import * as v from 'valibot';

export const variables = defineEnvVars({
	SERVER_URL: {},
	AUTH_AUTHENTIK_ISSUER: {
		schema: v.optional(v.string())
	},
	OAUTH_GROUPS: {
		schema: v.optional(v.string())
	},
	AUTH_SECRET: {
		schema: v.optional(v.string())
	},
	AUTH_AUTHENTIK_ID: {
		schema: v.optional(v.string())
	},
	AUTH_AUTHENTIK_SECRET: {
		schema: v.optional(v.string())
	}
});
