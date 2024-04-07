import type { Actions } from '@sveltejs/kit';
import { delete_action, update } from '$lib/server/actions';
import type { UpdateSchedule } from '$lib/api_bindings/update/UpdateSchedule';

const type = 'Schedule';

export const actions = {
	delete: async ({ params }) => await delete_action(type, params.uuid),
	update: async ({ params, request }) => {
		const body: UpdateSchedule = {
			name: '',
			playlist: '',
			scheduled: []
		};
		return await update({
			body,
			data: await request.formData(),
			type,
			uuid: params.uuid
		});
	}
} satisfies Actions;
