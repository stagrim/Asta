import { env } from '$env/dynamic/private';
import { building } from '$app/environment';
import { redirect, type Handle } from '@sveltejs/kit';
import { session_display_name, session_username, valid_session } from '$lib/server/auth';

if (env.SERVER_URL) {
	console.log(`Listening for Server on ${env.SERVER_URL}`);
} else if (!building) {
	throw new Error("SERVER_URL environment variable is not defined, can't connect to Server");
}

if (env.LDAP_URL) {
	console.log(`Listening to LDAP on ${env.LDAP_URL}`);
} else if (!building) {
	throw new Error("LDAP_URL environment variable is not defined, can't connect to LDAP");
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

export const handle: Handle = async ({ event, resolve }) => {
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

	const valid = await valid_session(
		event.cookies.get('session-id')!,
		event.request.headers.get('User-Agent')!
	);
	if (
		!event.url.pathname.startsWith('/login') &&
		!event.url.pathname.startsWith('/not-supported')
	) {
		if (valid) {
			event.locals.user = await session_username(event.cookies.get('session-id')!);
			event.locals.name = await session_display_name(event.cookies.get('session-id')!);
			// console.log("Valid req, will not redirect")
		} else {
			// console.log("Invalid req, will redirect to login")
			throw redirect(303, '/login');
		}
	} else if (event.url.pathname.startsWith('/login') && event.request.method === 'GET') {
		// Get requests to login sites should redirect to start page if user session is valid.
		// Logout is a Post request to login, so only GET should be reflected
		if (valid) {
			throw redirect(303, '/');
		}
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
