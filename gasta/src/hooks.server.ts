import { env } from '$env/dynamic/private';
import { building } from '$app/environment';
import { redirect, type Handle } from '@sveltejs/kit';
import { session_display_name, session_username, valid_session } from '$lib/server/auth';
import { authHandle } from './auth';
import { sequence } from '@sveltejs/kit/hooks';

if (env.SERVER_URL) {
	console.log(`Listening for Server on ${env.SERVER_URL}`);
} else if (!building) {
	throw new Error("SERVER_URL environment variable is not defined, can't connect to Server");
}

if (env.AUTH_AUTHENTIK_ISSUER) {
	console.log(`Using ODIC with endpoint on ${env.AUTH_AUTHENTIK_ISSUER}`);
} else if (!building) {
	throw new Error("AUTH_AUTHENTIK_ISSUER environment variable is not defined, can't connect to Authentik");
}

if (env.LDAP_GROUPS) {
	console.log(`LDAP groups allowed to log in: ${env.LDAP_GROUPS}`);
} else if (!building) {
	throw new Error(
		'LDAP_GROUPS environment variable is not defined, must specify groups allowed to log in'
	);
}

if (env.REDIS_URL) {
	console.log(`Listening to Redis on ${env.REDIS_URL}`);
} else if (!building) {
	throw new Error("REDIS_URL environment variable is not defined, can't connect to Redis");
}

export const defaultHandle: Handle = async ({ event, resolve }) => {
	// Ensure browser security
	if (
		!event.url.pathname.startsWith('/not-supported') &&
		event.request.headers.get('SEC-CH-UA')?.includes(`"Edge"`)
	) {
		throw redirect(303, '/not-supported');
	} else if (
		event.url.pathname.startsWith('/not-supported') &&
		!event.request.headers.get('SEC-CH-UA')?.includes(`"Edge"`)
	) {
		throw redirect(303, '/');
	}

	if (event.request.method === 'POST') {
		const clone = event.request.clone();
		const entries = [...(await clone.formData()).entries()];

		console.log(
			JSON.stringify(
				{
					type: 'POST request',
					name: await session_username(event.cookies.get('session-id')!),
					url: clone.url,
					body: entries
						.map(([k, v]) => (k === 'password' ? [k, '[redacted]'] : [k, v]))
						.reduce((prev, [key, val]) => {
							try {
								// Try to convert value to JSON and replace val with the parsed data
								val = JSON.parse(val.toString());
							} catch {
								console.log('Failed to parse: ' + val.toString());
							}
							return Object.assign(prev, { [key.toString()]: val });
						}, {})
				},
				null,
				2
			)
		);
	}

	return resolve(event);
};

export const handle = sequence(authHandle, defaultHandle);