import { form, command } from '$app/server';
import { logRequest } from '#lib/server/logger.ts';
import {
	createDisplay as apiCreateDisplay,
	updateDisplay as apiUpdateDisplay,
	deleteDisplay as apiDeleteDisplay
} from '#lib/server/sasta_client/sdk.gen.ts';
import { vCreateDisplayBody, vUpdateDisplayBody } from '#lib/server/sasta_client/valibot.gen.ts';
import { error, redirect } from '@sveltejs/kit';
import * as v from 'valibot';

const vPatchedCreateDisplayBody = v.object({
	...vCreateDisplayBody.entries,
	uuid: v.optional(
		v.pipe(
			v.string(),
			v.transform((s) => s || undefined),
			v.optional(v.pipe(v.string(), v.uuid()))
		)
	)
});
export const createDisplay = form(vPatchedCreateDisplayBody, async (body) => {
	await logRequest('createDisplay', body);
	const res = await apiCreateDisplay({
		body
	});
	if (res.error) {
		console.error('Failed to create display:', res.error);
		if (typeof res.error === 'string') {
			error(400, res.error || 'Failed to create display');
		}
		error(500, 'Failed to create display');
	}

	redirect(302, `/display/${res.data?.uuid}`);
});

export const updateDisplay = command(
	v.intersect([vUpdateDisplayBody, v.object({ uuid: v.string() })]),
	async ({ uuid, ...body }) => {
		await logRequest('updateDisplay', { uuid, ...body });
		const res = await apiUpdateDisplay({
			path: { uuid },
			body
		});
		if (res.error) {
			console.error('Failed to update display:', res.error);
			if (typeof res.error === 'string') {
				error(400, res.error || 'Failed to update display');
			}
			error(500, 'Failed to update display');
		}
		return res.data;
	}
);

export const deleteDisplay = command(v.object({ uuid: v.string() }), async ({ uuid }) => {
	await logRequest('deleteDisplay', { uuid });
	const res = await apiDeleteDisplay({ path: { uuid } });
	if (res.error) {
		console.error('Failed to delete display:', res.error);
		if (typeof res.error === 'string') {
			error(400, res.error || 'Failed to delete display');
		}
		error(500, 'Failed to delete display');
	}
});
