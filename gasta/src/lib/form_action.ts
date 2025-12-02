import { applyAction } from '$app/forms';
import { goto, invalidateAll } from '$app/navigation';
import { toast } from 'svelte-sonner';

// Set type is only to stop TS from complaining, could not find types for the function in SvelteKit
export const form_action: (_: any) => Promise<void> = async ({ result }) => {
	if (result.type === 'success') {
		if (result.data?.message) {
			toast(result.data.message, { duration: 2000 });
		}

		await invalidateAll();

		if (result.data?.redirect) {
			goto(result.data?.redirect);
		}

		await applyAction(result);
	} else if (result.type === 'failure') {
		toast(result.data.message);
	} else {
		toast(result.message ?? 'Something went wrong with the Request');
	}
};
