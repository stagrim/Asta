import { applyAction } from '$app/forms';
import { goto, invalidateAll } from '$app/navigation';
import { getToastStore } from '@skeletonlabs/skeleton';
import { get } from 'svelte/store';
import { toastStore } from './stores';

// Set type is only to stop TS from complaining, could not find types for the function in SvelteKit
export const form_action: (_: any) => Promise<void> = async ({ result }) => {
	if (result.type === 'success') {
		if (result.data?.message) {
			get(toastStore).trigger({
				message: result.data.message,
				background: 'variant-filled-success',
				timeout: 2000
			});
		}

		await invalidateAll();

		if (result.data?.redirect) {
			goto(result.data?.redirect);
		}

		await applyAction(result);
	} else if (result.type === 'failure') {
		get(toastStore).trigger({
			message: result.data.message,
			background: 'variant-filled-warning',
			autohide: false
		});
	} else {
		get(toastStore).trigger({
			message: result.message ?? 'Something went wrong with the Request',
			background: 'variant-filled-error',
			autohide: false
		});
	}
};
