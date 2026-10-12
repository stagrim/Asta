import type { DisplayState, PlaylistState, ScheduleState } from '../app';
import type { LayoutServerLoad } from './$types';
import {
	readDisplays,
	readPlaylist,
	readSchedules,
	type ReadDisplay,
	type ReadPlaylist,
	type ReadSchedule
} from '#lib/server/sasta_client/index.ts';
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
				content: new Map<string, ReadDisplay>()
			},
			schedule: {
				type: 'Schedule',
				content: new Map<string, ReadSchedule>()
			},
			playlist: {
				type: 'Playlist',
				content: new Map<string, ReadPlaylist>()
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
		readSchedules().then(arrayToMap),
		readPlaylist().then(arrayToMap)
	]);

	return {
		display: { content: display, type: 'Display' },
		schedule: { content: schedule, type: 'Schedule' },
		playlist: { content: playlist, type: 'Playlist' },
		user: session?.user.preferred_username
	};
}) satisfies LayoutServerLoad;
