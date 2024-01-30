import type { Actions } from '@sveltejs/kit';
import type { UpdateDisplay } from '../../../api_bindings/update/UpdateDisplay';
import { delete_action, update } from '../../../lib/server/actions';

export const actions = {
	delete: async ({ params }) => await delete_action('Display', params.uuid),
	update: async ({ params, request }) => {
		const body: UpdateDisplay = {
			name: '',
			schedule: ''
		};
		return await update({
			body,
			data: await request.formData(),
			type: 'Display',
			uuid: params.uuid
		});
	}
} satisfies Actions;
