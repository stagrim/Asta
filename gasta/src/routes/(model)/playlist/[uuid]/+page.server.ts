import type { Actions } from '@sveltejs/kit';
import { delete_action, update } from '$lib/server/actions';
import type { UpdatePlaylist } from '$lib/api_bindings/update/UpdatePlaylist';
import type { PageServerLoad } from './$types';

const type = 'Playlist';

export const load: PageServerLoad = async ({ params }) => {
	return { uuid: params.uuid };
};

export const actions = {
	delete: async ({ params }) => await delete_action(type, params.uuid),
	update: async ({ params, request }) => {
		const body: UpdatePlaylist = {
			name: '',
			items: []
		};
		return await update({
			body,
			data: await request.formData(),
			type,
			uuid: params.uuid
		});
	}
} satisfies Actions;
