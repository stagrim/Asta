// See https://kit.svelte.dev/docs/types#app
// for information about these interfaces
declare global {
	namespace App {
		// interface Error {}
		// interface Locals {}
		// interface PageData {}
		// interface Platform {}
		interface ImportMetaEnv {
			SERVER_URL: string
		}
	}
}

export type Uuid = String

export interface Content {
	uuid: Uuid,
	name: string,
}

export interface Display extends Content {
	schedule: Uuid
}

export interface Schedule extends Content {
	playlist: Uuid
}

export interface Playlist extends Content {
	playlist: Uuid
}

export interface Payload<C extends Content> {
    type: "Display",
    content: C[]
}

export interface State<C extends Content> extends Payload<C> {
    content: Map<Uuid, C>,
	values: C[]
}
