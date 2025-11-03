import { invalidate_session, login, session_username } from '$lib/server/auth';
import { fail, redirect } from '@sveltejs/kit';
import type { Actions, PageServerLoad } from './$types';
import process from "node:process";

export const load: PageServerLoad = async (event) => {
	const session = await event.locals.auth();
	let banner;
	if (process.env.NODE_ENV === 'development') {
		banner = `Development mode in use, admin account with password 'admin' is usable`;
	}

	if (session?.user) {
		redirect(302, "/");
	}

	return {banner, session}

};
