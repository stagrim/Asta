import { env } from '$env/dynamic/private';
import type { Payload } from '../api_bindings/read/Payload';
import type { State } from '../app';
import type { LayoutServerLoad } from './$types';
import type { Display } from '../api_bindings/read/Display';
import type { Playlist } from '../api_bindings/read/Playlist';
import type { Schedule } from '../api_bindings/read/Schedule';

export const load = (async ({ locals }) => {
	if (!locals.user) {
		return {
			display: {
				type: 'Display',
				content: new Map<string, Display>()
			},
			schedule: {
				type: 'Schedule',
				content: new Map<string, Schedule>()
			},
			playlist: {
				type: 'Playlist',
				content: new Map<string, Playlist>()
			},
			empty: true
		};
	}

	const get = async (api_route: string) => {
		const payload: Payload = await fetch(`${env.SERVER_URL}/api/${api_route}`).then((d) =>
			d.json()
		);

		if (payload.type == 'Error') {
			console.error(`Error: ${payload}`);
			throw Error();
		}

		const map = new Map();
		payload.content.forEach((c) => map.set(c.uuid, c));

		const res: State = {
			type: payload.type,
			content: map
		};
		return res;
	};

	const display: State = await get('display');
	if (display.type != 'Display') throw Error();
	const schedule: State = await get('schedule');
	if (schedule.type != 'Schedule') throw Error();
	const playlist: State = await get('playlist');
	if (playlist.type != 'Playlist') throw Error();
	return {
		display,
		schedule,
		playlist,
		user: locals.user
	};
}) satisfies LayoutServerLoad;
