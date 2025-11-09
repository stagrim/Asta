import { env } from '$env/dynamic/private';
import { building } from '$app/environment';
import { redirect, type Handle } from '@sveltejs/kit';
import { authHandle } from '$lib/auth';
import { sequence } from '@sveltejs/kit/hooks';

if (env.SERVER_URL) {
	console.log(`Listening for Server on ${env.SERVER_URL}`);
} else if (!building) {
	throw new Error("SERVER_URL environment variable is not defined, can't connect to Server");
}

if (env.AUTH_AUTHENTIK_ISSUER) {
	console.log(`Using ODIC with endpoint on ${env.AUTH_AUTHENTIK_ISSUER}`);
} else if (!building) {
	throw new Error(
		"AUTH_AUTHENTIK_ISSUER environment variable is not defined, can't connect to Authentik"
	);
}

if (env.OAUTH_GROUPS) {
	console.log(`OAUTH groups allowed to log in: ${env.OAUTH_GROUPS}`);
} else if (!building) {
	console.log('OAUTH groups not set, anybody can log in');
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

	const session = await event.locals.auth();
	if (
		!event.url.pathname.startsWith('/login') &&
		// TODO: Make a error page instead?
		!event.url.pathname.startsWith('/not-authorized') &&
		!event.url.pathname.startsWith('/not-supported')
	) {
		if (!session) {
			throw redirect(303, '/login');
		}
	} else if (
		(event.url.pathname.startsWith('/login') || event.url.pathname.startsWith('/not-authorized')) &&
		event.request.method === 'GET'
	) {
		// Get requests to login sites should redirect to start page if user session is valid.
		// Logout is a Post request to login, so only GET should be reflected
		if (session) {
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
					name: session?.user,
					url: clone.url,
					body: entries.reduce((prev, [key, val]) => {
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
