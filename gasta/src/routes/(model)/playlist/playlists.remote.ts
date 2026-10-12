import { form, command } from '$app/server';
import { logRequest } from '#lib/server/logger.ts';
import {
	createPlaylist as apiCreatePlaylist,
	updatePlaylist as apiUpdatePlaylist,
	deletePlaylist as apiDeletePlaylist
} from '#lib/server/sasta_client/sdk.gen.ts';
import { vCreatePlaylistBody, vUpdatePlaylistBody } from '#lib/server/sasta_client/valibot.gen.ts';
import { error, redirect } from '@sveltejs/kit';
import * as v from 'valibot';

export const createPlaylist = form(vCreatePlaylistBody, async (body) => {
	await logRequest('createPlaylist', body);
	const res = await apiCreatePlaylist({ body });
	if (res.error) {
		console.error('Failed to create playlist:', res.error);
		if (typeof res.error === 'string') {
			error(400, res.error || 'Failed to create playlist');
		}
		error(500, 'Failed to create playlist');
	}

	redirect(302, `/playlist/${res.data?.uuid}`);
});

export const updatePlaylist = command(
	v.intersect([vUpdatePlaylistBody, v.object({ uuid: v.string() })]),
	async ({ uuid, ...body }) => {
		await logRequest('updatePlaylist', { uuid, ...body });
		const res = await apiUpdatePlaylist({
			path: { uuid },
			body: {
				...body,
				// @ts-ignore - The generated OpenAPI TS interface uses 'number', but Valibot parses it as 'bigint'. We map it to Number.
				items: body.items.map((item) => {
					const newItem = { ...item };
					// @ts-expect-error: API client typed duration as number, but valibot parsed as bigint
					newItem.settings.duration = Number(item.settings.duration);
					// @ts-ignore - The generated OpenAPI TS interface uses 'number', but Valibot parses it as 'bigint'. We map it to Number.
					return newItem;
				})
			}
		});
		if (res.error) {
			console.error('Failed to update playlist:', res.error);
			if (typeof res.error === 'string') {
				error(400, res.error || 'Failed to update playlist');
			}
			error(500, 'Failed to update playlist');
		}
		return res.data;
	}
);

export const deletePlaylist = command(v.object({ uuid: v.string() }), async ({ uuid }) => {
	await logRequest('deletePlaylist', { uuid });
	const res = await apiDeletePlaylist({ path: { uuid } });
	if (res.error) {
		console.error('Failed to delete playlist:', res.error);
		if (typeof res.error === 'string') {
			error(400, res.error || 'Failed to delete playlist');
		}
		error(500, 'Failed to delete playlist');
	}
});
