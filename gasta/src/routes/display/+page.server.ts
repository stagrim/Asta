import type { Actions } from '@sveltejs/kit';
import type { CreateDisplay } from '../../api_bindings/create/CreateDisplay';
import { create } from '$lib/server/actions';

export const actions = {
	create: async ({ request }) => {
		const data = await request.formData();

		const body: CreateDisplay = {
			name: '',
			schedule: ''
		};
		if (data.get('uuid')) {
			// TODO: check Uuid validity here for friendlier error message
			body['uuid'] = '';
		}
		return await create({
			body,
			type: 'Display',
			data
		});
	}
} satisfies Actions;
