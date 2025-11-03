<script lang="ts">
	import { enhance } from '$app/forms';
	import type { ActionData, PageData } from './$types';
	import { toastStore } from '$lib/stores';
	import { SignIn } from '@auth/sveltekit/components';

	export let data: PageData;

	export let form: ActionData;

	$: if (form?.msg) {
		$toastStore.trigger({
			message: form.msg,
			background: 'variant-filled-error',
			autohide: false
		});
	}
</script>

{#if data?.banner}
	<div class="variant-filled-warning text-center">
		{data.banner}
	</div>
{/if}

<div class="flex justify-center">
	<div class="card m-4 max-w-4xl">
		<section class="p-4 text-center">
			<h1 class="font-bold text-3xl p-3">
				Welcome to <span
					class="bg-gradient-to-br from-primary-500 to-primary-300 bg-clip-text text-transparent box-decoration-clone"
				>
					Asta
				</span>
			</h1>
			<SignIn provider="authentik">
				<span class="btn variant-filled-primary" slot="submitButton">Sign in using D-sektionen</span
				>
			</SignIn>
		</section>
	</div>
</div>
