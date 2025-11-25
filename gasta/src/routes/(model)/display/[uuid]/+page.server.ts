import type { Actions } from '@sveltejs/kit';
import type { UpdateDisplay } from '$lib/api_bindings/update/UpdateDisplay';
import { delete_action, update } from '$lib/server/actions';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = ({ params }) => {
	return { uuid: params.uuid };
};

export const actions = {
	delete: async ({ params }) => await delete_action('Display', params.uuid),
	update: async ({ params, request }) => {
		const body: UpdateDisplay = {
			name: '',
			display_material: { type: 'schedule', uuid: '' }
		};
		return await update({
			body,
			data: await request.formData(),
			type: 'Display',
			uuid: params.uuid
		});
	}
} satisfies Actions;
