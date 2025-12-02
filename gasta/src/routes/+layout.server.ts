import { env } from '$env/dynamic/private';
import type { Payload } from '$lib/api_bindings/read/Payload';
import type { DisplayState, PlaylistState, ScheduleState, State } from '../app';
import type { LayoutServerLoad } from './$types';
import type { Display } from '$lib/api_bindings/read/Display';
import type { Playlist } from '$lib/api_bindings/read/Playlist';
import type { Schedule } from '$lib/api_bindings/read/Schedule';

export const load = (async ({
	locals
}): Promise<{
	display: DisplayState;
	schedule: ScheduleState;
	playlist: PlaylistState;
	empty?: boolean;
	user?: string;
}> => {
	const session = await locals.auth();

	if (!session || !session.user) {
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

	const get = async (api_route: string): Promise<DisplayState | PlaylistState | ScheduleState> => {
		const payload: Payload = await fetch(`${env.SERVER_URL}/api/${api_route}`).then((d) =>
			d.json()
		);

		if (payload.type == 'Error') {
			console.error(`Error: ${payload}`);
			throw Error();
		}

		const map = new Map();
		payload.content.forEach((c) => map.set(c.uuid, c));

		return {
			type: payload.type,
			content: map
		};
	};

	const display = await get('display');
	if (display.type != 'Display') throw Error();
	const schedule = await get('schedule');
	if (schedule.type != 'Schedule') throw Error();
	const playlist = await get('playlist');
	if (playlist.type != 'Playlist') throw Error();

	schedule.content.forEach((v) => v.scheduled?.forEach((s) => (s.id = crypto.randomUUID())));

	return {
		display,
		schedule,
		playlist,
		user: session?.user.preferred_username
	};
}) satisfies LayoutServerLoad;
