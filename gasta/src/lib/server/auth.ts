import { env } from '$env/dynamic/private';
import ldapjs, { type Client } from 'ldapjs';
const { createClient } = ldapjs;
import pkg from 'js-sha3';
import { building, dev } from '$app/environment';
import type { CookieSerializeOptions } from 'cookie';
import { Redis } from 'ioredis';
const { sha3_512 } = pkg;

export type Login =
	| {
			result: 'success';
			session_id: string;
			cookie: CookieSerializeOptions & { path: string };
	  }
	| {
			result: 'failure';
			msg: string;
	  };

export interface Session {
	// session_id: string,
	name: string;
	username: string;
	user_agent: string;
	// expires: string,
}

// TODO: Set setInterval to remove old sessions from map

const session_valid_time = 60 * 60 * 24 * 7; // Valid for a week

let redis: Redis;
let ldap: Client;

if (!building) {
	ldap = createClient({
		url: [env.LDAP_URL],
		timeout: 2000,
		connectTimeout: 2000,
		reconnect: true
	});

	ldap.on('error', (err) => {
		console.debug({ msg: 'connection failed, retrying', err });
	});

	redis = new Redis(env.REDIS_URL);
}

export const login = async (
	username: string,
	password: string,
	user_agent: string
): Promise<Login> => {
	let res: Login;
	let auth: Authenticate = await authenticate(username, password);

	if (
		auth.result === 'success' ||
		(process.env.NODE_ENV === 'development' && username === 'admin' && password === 'admin')
	) {
		const message = `${user_agent}${Math.random()}${Date.now()}`;
		const session_id = sha3_512(`${env.UUID5_NAMESPACE}${username}${message}`);
		let session: Session = {
			user_agent,
			username,
			name: auth.result === 'success' ? auth.name : 'Rosa Pantern'
		};
		if (await redis.exists(session_id)) {
			res = {
				result: 'failure',
				msg: 'Session id is already in use??? (contact person responsible)'
			};
		} else {
			redis.set(session_id, JSON.stringify(session));
			redis.expire(session_id, session_valid_time);
			res = {
				result: 'success',
				session_id,
				cookie: {
					path: '/',
					httpOnly: true,
					secure: !dev,
					sameSite: 'strict',
					maxAge: session_valid_time
				}
			};
		}
	} else {
		res = auth;
	}
	return res;
};

export const valid_session = async (session_id: string, user_agent: string): Promise<boolean> => {
	const str = await redis.get(session_id);
	if (!str) return false;
	const session: Session = JSON.parse(str);
	return session.user_agent == user_agent;
};

export const session_username = async (session_id: string): Promise<string> => {
	const str = await redis.get(session_id);
	if (!str) return '';
	const session: Session = JSON.parse(str);
	return session.username;
};

export const session_display_name = async (session_id: string): Promise<string> => {
	const str = await redis.get(session_id);
	if (!str) return '';
	const session: Session = JSON.parse(str);
	return session.name;
};

export const invalidate_session = async (session_id: string) => await redis.del(session_id);

export type Authenticate =
	| {
			result: 'success';
			name: string;
	  }
	| {
			result: 'failure';
			msg: string;
	  };

// (|(*group logic*)(*another group logic*))
const filter = `(|${['dsek.km', 'dsek.cafe', 'dsek.sex']
	.map((g) => `(memberOf=cn=${g},cn=groups,cn=accounts,dc=dsek,dc=se)`)
	.join('')})`;

const authenticate = async (username: string, password: string): Promise<Authenticate> => {
	return new Promise(function (resolve) {
		// Oskar "badoddss" Stenberg was here
		if (!new RegExp('^[A-Za-z0-9]{6,10}(-s)?$').test(username)) {
			resolve({
				result: 'failure',
				msg: `Ye ain't smart enough to remember yer stil-id, harr harr!`
			});
		}

		let res: string[] = [];

		ldap.bind(`uid=${username},cn=users,cn=accounts,dc=dsek,dc=se`, password, (err, _) => {
			if (err) {
				console.log({ err: err.name });
				ldap.unbind();
				if (err.name === 'ConnectionError') {
					resolve({
						result: 'failure',
						msg: `Aye be tryin' to reach the LDAP server, but it be as elusive as buried treasure on a deserted island!`
					});
				} else if (err.name === 'InvalidCredentialsError') {
					resolve({
						result: 'failure',
						msg: `Ye 'ave forgotten yer username or yer password`
					});
				} else {
					resolve({
						result: 'failure',
						msg: `Arrr, ye scallywag! We've hit a rough sea on the LDAP voyage - ${err.name}, the treasure map to that directory be lost to the depths of Davy Jones' locker!`
					});
				}
			} else {
				ldap.search(
					`uid=${username},cn=users,cn=accounts,dc=dsek,dc=se`,
					{},
					(searchError, searchResponse) => {
						if (searchError) {
							console.error('LDAP search error:', searchError);
							ldap.unbind();
							resolve({
								result: 'failure',
								msg: `LDAP error: ${searchError.message}`
							});
						}
						searchResponse.on('searchEntry', (entry) => {
							// The entry object contains information about the group
							const display_name =
								entry.attributes.find((a) => a.type === 'givenName')?.values[0] ?? '';
							res.push(display_name);
						});
						searchResponse.on('end', () => {
							ldap.unbind();
							if (res.length > 0) {
								resolve({
									result: 'success',
									name: res[0]
								});
							} else {
								resolve({
									result: 'failure',
									msg: "Ahoy there, matey! I be sorry to inform ye that ye don't have the proper authorization to be layin' eyes on the Asta web page. Arrr, it be guarded like a chest of precious booty, and only them with the right permissions can set their sights on it. Ye best be seekin' permission from the rightful owner or the webmaster if ye wish to gain access to that there treasure trove of information. Fair winds to ye on yer digital adventures, but for now, ye best be sailin' away from these waters. Arrr! 🏴‍☠️⚓🦜"
								});
							}
						});

						searchResponse.on('connectError', (err) => {
							console.log(err);
							resolve({
								result: 'failure',
								msg: `LDAP error: ${err.message}`
							});
						});
					}
				);
			}
		});
	});
};
