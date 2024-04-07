import type { Actions } from '@sveltejs/kit';
import type { CreatePlaylist } from '$lib/api_bindings/create/CreatePlaylist';
import { create } from '$lib/server/actions';

export const actions = {
	create: async ({ request }) => {
		const body: CreatePlaylist = {
			name: ''
		};
		return await create({
			body,
			type: 'Playlist',
			data: await request.formData()
		});
	}
} satisfies Actions;
