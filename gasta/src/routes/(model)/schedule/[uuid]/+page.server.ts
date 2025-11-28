import type { Actions } from '@sveltejs/kit';
import { delete_action, update } from '$lib/server/actions';
import type { UpdateSchedule } from '$lib/api_bindings/update/UpdateSchedule';
import type { ScheduleInfo } from '$lib/api_bindings/read/ScheduleInfo';
import { env } from '$env/dynamic/private';
import type { PageServerLoad } from './$types';

const type = 'Schedule';

export const load: PageServerLoad = async ({ params }) => {
	// TODO: await on client instead?
	const schedule_info: ScheduleInfo = await (
		await fetch(`${env.SERVER_URL}/api/${type.toLocaleLowerCase()}/${params.uuid}`)
	).json();

	return {
		schedule_info,
		uuid: params.uuid
	};
};

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
