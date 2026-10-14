import { getRequestEvent } from '$app/server';

export async function logRequest(action: string, body?: unknown) {
	try {
		const event = getRequestEvent();
		const session = await event.locals.auth();

		console.log(
			JSON.stringify(
				{
					type: 'Remote request',
					action,
					name: session?.user,
					url: event.url.href,
					body
				},
				(_key, value) => {
					if (typeof value === 'bigint') {
						return Number(value);
					}
					if (value instanceof File) {
						return {
							name: value.name,
							size: value.size,
							type: value.type
						};
					}
					return value;
				},
				2
			)
		);
	} catch (e) {
		console.error(`Could not log remote request '${action}': ${e}`);
	}
}
