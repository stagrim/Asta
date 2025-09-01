import { fail, type Actions } from '@sveltejs/kit';
import type { CreateDisplay } from '$lib/api_bindings/create/CreateDisplay';
import { create, return_handling } from '$lib/server/actions';
import { env } from '$env/dynamic/private';

interface FormCreateDisplay {
	name: string;
	uuid?: string;
	display_type: 'schedule' | 'playlist';
	display_uuid: string;
}

export const actions = {
	create: async ({ request }) => {
		const type = 'Display';
		const data = await request.formData();

		const body: FormCreateDisplay & { [k: string]: undefined | string | object } = {
			name: '',
			display_type: 'schedule',
			display_uuid: ''
		};

		if (data.get('uuid')) {
			// TODO: check Uuid validity here for friendlier error message
			body['uuid'] = '';
		}

		for (const key of Object.keys(body)) {
			const field = data.get(key);
			if (field) {
				body[key] = field.toString();
			} else {
				return fail(400, { message: `The field ${key} is empty` });
			}
		}

		const createDisplay: CreateDisplay = {
			name: body.name,
			uuid: body.uuid,
			display_material: { type: body.display_type, uuid: body.display_uuid }
		};

		const res = await fetch(`${env.SERVER_URL}/api/${type.toLocaleLowerCase()}`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify(createDisplay)
		});
		return await return_handling({
			res,
			type,
			ret: (uuid) => ({
				redirect: `/${type.toLocaleLowerCase()}/${uuid}`,
				message: `${type} Added`
			})
		});
	}
} satisfies Actions;
