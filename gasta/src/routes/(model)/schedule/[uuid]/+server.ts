import { env } from '$env/dynamic/private';
import type { ScheduleInfo } from '$lib/api_bindings/read/ScheduleInfo';
import { json } from '@sveltejs/kit';

export async function GET({ params }) {
	const schedule_info: ScheduleInfo = await (
		await fetch(`${env.SERVER_URL}/api/schedule/${params.uuid}`)
	).json();

	return json(schedule_info);
}
