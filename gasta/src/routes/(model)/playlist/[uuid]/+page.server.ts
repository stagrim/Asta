import type { Actions } from '@sveltejs/kit';
import { delete_action, update } from '$lib/server/actions';
import type { UpdatePlaylist } from '$lib/api_bindings/update/UpdatePlaylist';
import type { PageServerLoad } from './$types';
import { getAllPathsTree } from '$lib/server/sasta_client';

const type = 'Playlist';

export const load: PageServerLoad = async ({ params }) => {
	const files = await getAllPathsTree();
	if (files.error || !files.data) throw new Error('Could not load files list');
	return { uuid: params.uuid, files: files.data };
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
