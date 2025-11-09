import { SvelteKitAuth, type DefaultSession } from '@auth/sveltekit';
import Authentik, { type AuthentikProfile } from '@auth/sveltekit/providers/authentik';
import * as dotenv from 'dotenv';
import { env } from '$env/dynamic/private';
import process from 'node:process';
import Credentials from '@auth/sveltekit/providers/credentials';
import type { Provider } from '@auth/sveltekit/providers';
import { isInDevEnvironment } from './utils';

dotenv.config();

declare module '@auth/sveltekit' {
	interface Session {
		user: {
			preferred_username: string;
			name: string;
			userId: string;
			group_list: string[];
		} & DefaultSession['user'];
	}
}

let providers: Provider[] = [
	Authentik({
		clientId: process.env.AUTH_AUTHENTIK_ID,
		clientSecret: process.env.AUTH_AUTHENTIK_SECRET,
		issuer: process.env.AUTH_AUTHENTIK_ISSUER,
		profile: (profile: AuthentikProfile) => {
			return {
				userId: profile.preferred_username,
				name: profile.name,
				group_list: profile['groups'] ?? []
			};
		}
	})
];

// Activate Mock login
if (isInDevEnvironment) {
	providers.push(
		Credentials({
			async authorize() {
				return {
					preferred_username: 'rosapantern',
					userId: 'rosapantern',
					name: 'Rosa Pantern',
					group_list: []
				};
			}
		})
	);
}

export const {
	handle: authHandle,
	signIn,
	signOut
} = SvelteKitAuth({
	trustHost: true,
	secret: process.env.AUTH_SECRET,
	providers,
	callbacks: {
		signIn({ profile }) {
			console.log({ action: 'Tries to login', profile });

			const userGroups = (profile?.groups as string[] | undefined) ?? [];
			const adminGroups = (env.OAUTH_GROUPS ?? '')
				.split(',')
				.map((i) => i.trim())
				.filter((i) => i);

			return (
				adminGroups.length == 0 ||
				adminGroups.some((g1) => userGroups.some((g2) => g2.startsWith(g1)))
			);
		},
		jwt({ token, user }) {
			if (user) {
				// User is available during sign-in
				// eslint-disable-next-line @typescript-eslint/ban-ts-comment
				// @ts-ignore
				token.userId = user.userId;
				// eslint-disable-next-line @typescript-eslint/ban-ts-comment
				// @ts-ignore
				token.group_list = (user.group_list as string[] | undefined) ?? [];
			}
			return token;
		},
		session({ session, token }) {
			session.user.userId = token.userId as string;
			session.user.group_list = (token.group_list as string[] | undefined) ?? [];

			return session;
		}
	},
	pages: {
		error: '/not-authorized'
	}
});
