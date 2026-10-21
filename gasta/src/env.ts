import { defineEnvVars } from '@sveltejs/kit/env';
import { dev } from '$app/env';
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
	},
	AUTH_USE_SECURE_COOKIES: {
		// Defaults to false in dev and true in production
		static: !dev,
		schema: v.pipe(
			v.optional(v.string(), ''),
			v.transform((str) => {
				if (str === undefined || str === '') {
					return !dev;
				}
				return str === 'true' || str === '1';
			})
		)
	}
});
