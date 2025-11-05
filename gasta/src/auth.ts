import { SvelteKitAuth, type DefaultSession } from '@auth/sveltekit';
import Authentik, { type AuthentikProfile } from '@auth/sveltekit/providers/authentik';
import * as dotenv from 'dotenv';

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

export const {
	handle: authHandle,
	signIn,
	signOut
} = SvelteKitAuth({
	trustHost: true,
	secret: process.env.AUTH_SECRET,
	providers: [
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
	],

	callbacks: {
        signIn({profile}) {
            const adminGroups = process.env.OAUTH_GROUPS?.split(' ') as string[];
            let userGroups = profile?.groups as string[];
	        let userGroupList = userGroups.map((s) => s.replace('/', '')).toString();
            return (adminGroups && adminGroups.some((g) => userGroupList.includes(g)));
        },
		jwt({ token, user }) {
			if (user) {
				// User is available during sign-in
				// eslint-disable-next-line @typescript-eslint/ban-ts-comment
				// @ts-ignore
				token.userId = user.userId;
				// eslint-disable-next-line @typescript-eslint/ban-ts-comment
				// @ts-ignore
				token.group_list = user.group_list ?? [];
			}
			return token;
		},
		session({ session, token }) {
			session.user.userId = token.userId as string;
			session.user.group_list = token.group_list as string[];

			return session;
		}
	},
    pages: {
        error: "/error"
    }
});