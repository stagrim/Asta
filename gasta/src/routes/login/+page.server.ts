import { invalidate_session, login, session_username } from '$lib/server/auth';
import { fail, redirect } from '@sveltejs/kit';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = async () => {
	if (process.env.NODE_ENV === 'development')
		return { banner: `Development mode in use, admin account with password 'admin' is usable` };
};

export const actions = {
	login: async ({ request, cookies }) => {
		const data = await request.formData();
		const username = data.get('username')?.toString();
		const password = data.get('password')?.toString();
		const user_agent = request.headers.get('User-Agent')?.toString();

		if (!username) {
			return fail(400, { msg: 'Username required' });
		}
		if (!password) {
			return fail(400, { msg: 'Passwords required' });
		}
		if (!user_agent) {
			return fail(400, { msg: 'User Agent required' });
		}

		const res = await login(username, password, user_agent);

		if (res.result === 'success') {
			console.log({ type: 'login', name: username, message: 'Successful login' });

			cookies.set('session-id', res.session_id, res.cookie);
		} else {
			console.log({ type: 'login', name: username, message: 'Login attempt rejected' });
			return { msg: res.msg, username };
		}
		throw redirect(303, '/');
	},
	logout: ({ cookies }) => {
		const username = session_username(cookies.get('session-id')!);
		invalidate_session(cookies.get('session-id')!);
		cookies.delete('session-id', { path: '/' });
		console.log({ type: 'logout', name: username, message: 'Successful logout' });
		throw redirect(303, '/login');
	}
} satisfies Actions;
