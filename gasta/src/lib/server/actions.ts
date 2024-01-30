import { env } from '$env/dynamic/private';
import { fail, redirect } from '@sveltejs/kit';
import type { Payload } from '../../api_bindings/read/Payload';

export type type = 'Display' | 'Playlist' | 'Schedule';

interface Input {
	body: { [key: string]: any };
	type: type;
	data: FormData;
	uuid?: string;
}

const return_handling = async ({
	res: response,
	type: type,
	ret
}: {
	res: Response;
	type: type;
	ret: (uuid: string) => {
		message: string;
		redirect?: string;
	};
}) => {
	const text = await response.text();
	let payload: Payload;
	try {
		payload = JSON.parse(text);
		console.log(payload);
	} catch {
		console.log(text);
		return fail(400, { message: text });
	}

	if (payload.type == type) {
		return ret(payload.content[0].uuid);
	} else if (payload.type == 'Error') {
		return fail(400, { message: payload.content.message });
	} else {
		return fail(400, { message: text });
	}
};

export const create = async ({ body, type, data }: Input) => {
	for (const key of Object.keys(body)) {
		const field = data.get(key);
		if (field) {
			body[key] = field.toString();
		} else {
			return fail(400, { message: `The field ${key} is empty` });
		}
	}

	const res = await fetch(`${env.SERVER_URL}/api/${type.toLocaleLowerCase()}`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(body)
	});

	return await return_handling({
		res,
		type,
		ret: (uuid) => ({ redirect: `/${type.toLocaleLowerCase()}/${uuid}`, message: `${type} Added` })
	});
};

export const update = async ({ body, data, type, uuid }: Input) => {
	if (!uuid) return fail(400, { message: `Missing Uuid` });

	for (const key of Object.keys(body)) {
		const field = data.get(key);
		if (field) {
			try {
				body[key] = JSON.parse(field.toString());
			} catch (e) {
				body[key] = field.toString();
			}
		} else {
			return fail(400, { message: `${key} field was empty` });
		}
	}

	console.log(JSON.stringify(body));

	const res = await fetch(`${env.SERVER_URL}/api/${type.toLocaleLowerCase()}/${uuid}`, {
		method: 'PUT',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(body)
	});

	return await return_handling({
		res,
		type,
		ret: (_) => ({ message: 'Display updated' })
	});
};

export const delete_action = async (type: type, uuid?: string) => {
	if (!uuid) return fail(400, { message: `Missing Uuid` });

	const res = await fetch(`${env.SERVER_URL}/api/${type.toLocaleLowerCase()}/${uuid}`, {
		method: 'DELETE'
	});

	return await return_handling({
		res,
		type,
		ret: (_) => ({ redirect: `/${type.toLocaleLowerCase()}`, message: `${type} Deleted` })
	});
};
