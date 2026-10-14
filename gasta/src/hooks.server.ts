import { SERVER_URL, AUTH_AUTHENTIK_ISSUER, OAUTH_GROUPS } from '$app/env/private';
import { building } from '$app/env';
import { redirect, type Handle } from '@sveltejs/kit';
import { authHandle } from '#lib/auth.ts';
import { sequence } from '@sveltejs/kit/hooks';

if (SERVER_URL) {
	console.log(`Listening for Server on ${SERVER_URL}`);
} else if (!building) {
	throw new Error("SERVER_URL environment variable is not defined, can't connect to Server");
}

if (AUTH_AUTHENTIK_ISSUER) {
	console.log(`Using ODIC with endpoint on ${AUTH_AUTHENTIK_ISSUER}`);
} else if (!building) {
	console.log(
		"AUTH_AUTHENTIK_ISSUER environment variable is not defined, can't connect to Authentik"
	);
}

if (OAUTH_GROUPS) {
	console.log(`OAUTH groups allowed to log in: ${OAUTH_GROUPS}`);
} else if (!building) {
	console.log('OAUTH groups not set, anybody can log in');
}

export const defaultHandle: Handle = async ({ event, resolve }) => {
	const session = await event.locals.auth();
	if (
		!event.url.pathname.startsWith('/login') &&
		// TODO: Make an error page instead?
		!event.url.pathname.startsWith('/not-authorized')
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

	return resolve(event);
};

export const handle = sequence(authHandle, defaultHandle);
