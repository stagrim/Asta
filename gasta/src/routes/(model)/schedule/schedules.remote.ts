import { command, form } from '$app/server';
import { logRequest } from '#lib/server/logger.ts';
import {
	createSchedule as apiCreateSchedule,
	deleteSchedule as apiDeleteSchedule,
	scheduleInfo as apiScheduleInfo,
	updateSchedule as apiUpdateSchedule
} from '#lib/server/sasta_client/sdk.gen.ts';
import {
	vCreateScheduleBody,
	vScheduleInfoPath,
	vUpdateScheduleBody
} from '#lib/server/sasta_client/valibot.gen.ts';
import { error, redirect } from '@sveltejs/kit';
import * as v from 'valibot';

export const scheduleInfo = command(vScheduleInfoPath, async (path) => {
	const res = await apiScheduleInfo({ path });
	if (res.error) {
		console.error('Failed to fetch schedule info:', res.error);
		if (typeof res.error === 'string') {
			error(400, res.error || 'Failed to fetch schedule info');
		}
		error(500, 'Failed to fetch schedule info');
	}
	return res.data;
});

export const createSchedule = form(vCreateScheduleBody, async (body) => {
	await logRequest('createSchedule', body);
	const res = await apiCreateSchedule({ body });
	if (res.error) {
		console.error('Failed to create schedule:', res.error);
		if (typeof res.error === 'string') {
			error(400, res.error || 'Failed to create schedule');
		}
		error(500, 'Failed to create schedule');
	}

	redirect(302, `/schedule/${res.data?.uuid}`);
});

export const updateSchedule = command(
	v.intersect([vUpdateScheduleBody, v.object({ uuid: v.string() })]),
	async ({ uuid, ...body }) => {
		await logRequest('updateSchedule', { uuid, ...body });
		const res = await apiUpdateSchedule({
			path: { uuid },
			body
		});
		if (res.error) {
			console.error('Failed to update schedule:', res.error);
			if (typeof res.error === 'string') {
				error(400, res.error || 'Failed to update schedule');
			}
			error(500, 'Failed to update schedule');
		}

		return res.data;
	}
);

export const deleteSchedule = command(v.object({ uuid: v.string() }), async ({ uuid }) => {
	await logRequest('deleteSchedule', { uuid });
	const res = await apiDeleteSchedule({ path: { uuid } });
	if (res.error) {
		console.error('Failed to delete schedule:', res.error);
		if (typeof res.error === 'string') {
			error(400, res.error || 'Failed to delete schedule');
		}
		error(500, 'Failed to delete schedule');
	}
});
