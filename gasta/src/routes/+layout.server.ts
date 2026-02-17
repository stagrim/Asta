import type { DisplayState, PlaylistState, ScheduleState } from '../app';
import type { LayoutServerLoad } from './$types';
import {
	readDisplays,
	readPlaylist,
	readSchedule,
	type Display,
	type Playlist,
	type Schedule
} from '$lib/server/sasta_client';
import { error } from '@sveltejs/kit';

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

	function arrayToMap<T extends { uuid: string }>(res: { data?: T[]; error?: unknown }) {
		if (res.error || !res.data) {
			console.error('Error fetching state:', res.error);
			throw error(500, 'Could not fetch state');
		}
		return new Map(res.data.map((item) => [item.uuid, item]));
	}

	const [display, schedule, playlist] = await Promise.all([
		readDisplays().then(arrayToMap),
		readSchedule().then(arrayToMap),
		readPlaylist().then(arrayToMap)
	]);

	//TODO: Stupid fix; make data have ids in the db instead
	schedule.forEach((v) => v.scheduled?.forEach((s) => (s.id = crypto.randomUUID())));

	return {
		display: { content: display, type: 'Display' },
		schedule: { content: schedule, type: 'Schedule' },
		playlist: { content: playlist, type: 'Playlist' },
		user: session?.user.preferred_username
	};
}) satisfies LayoutServerLoad;
