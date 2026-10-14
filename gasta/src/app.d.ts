// See https://kit.svelte.dev/docs/types#app

import type { ReadDisplay, ReadPlaylist, ReadSchedule } from '#lib/server/sasta_client/index.ts';

// for information about these interfaces
declare global {
	namespace App {
		// interface Error {}
		// interface Locals {}
		// interface PageData {}
		// interface Platform {}
		interface Locals {
			user: string;
			name: string;
		}
	}
}
export type StateMap = {
	Display: ReadDisplay;
	Playlist: ReadPlaylist;
	Schedule: ReadSchedule;
};

export type StateTypes = keyof StateMap;

export type StateOf<K extends StateTypes = StateTypes> = {
	[P in K]: {
		type: P;
		content: Map<string, StateMap[P]>;
	};
}[K];

export type DisplayState = StateOf<'Display'>;
export type PlaylistState = StateOf<'Playlist'>;
export type ScheduleState = StateOf<'Schedule'>;

export type State = StateOf;
