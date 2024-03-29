<script lang="ts">
	import copy from 'svelte-awesome/icons/copy';

	import { page } from '$app/stores';
	import { getModalStore } from '@skeletonlabs/skeleton';
	import type { PageData } from './$types';
	import TypePicker from '../../../lib/TypePicker.svelte';
	import { enhance } from '$app/forms';
	import { form_action } from '$lib/form_action';
	import { toastStore } from '$lib/stores';
	import { Icon } from 'svelte-awesome';

	export let data: PageData;

	const modalStore = getModalStore();

	$: uuid = $page.params.uuid;
	const get_display = (uuid) => structuredClone(data.display.content.get(uuid));
	$: display = get_display(uuid);

	let delete_button: HTMLButtonElement;
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<!-- svelte-ignore a11y-no-static-element-interactions -->
<!-- svelte-ignore a11y-missing-attribute -->
<form class="card m-4" method="POST" use:enhance={() => form_action}>
	<section class="p-4">
		<span class="mb-5">
			<span>Uuid</span>
			<div
				class="input-group input-group-divider grid-cols-[1fr_auto] cursor-pointer"
				on:click={async (_) => {
					try {
						await navigator.clipboard.writeText(display.uuid);
						$toastStore.trigger({
							message: 'Display Uuid copied to clipboard',
							background: 'variant-filled-primary',
							timeout: 2000,
							hideDismiss: true
						});
					} catch (err) {
						$toastStore.trigger({
							message: 'Could not copy Uuid, ' + err,
							background: 'variant-filled-error',
							autohide: false
						});
					}
				}}
			>
				<input class="input text-surface-400" type="text" readonly value={display.uuid} />
				<a><Icon data={copy} scale={0.75} /></a>
			</div>
		</span>

		<label class="label mb-5">
			<span>Name</span>
			<input
				required
				name="name"
				class="input"
				type="text"
				placeholder="Name must be unique"
				bind:value={display.name}
			/>
		</label>

		<TypePicker name="schedule" bind:chosen_type={display.schedule} types={data.schedule} />

		<div class="flex w-full justify-center gap-4 mt-5">
			<button
				type="button"
				class="btn variant-ringed-error"
				on:click={() =>
					modalStore.trigger({
						type: 'confirm',
						title: `Delete '${display.name}'?`,
						body: `Are your sure you want to delete Display '${display.name}'?`,
						response: (r) => (r ? delete_button.click() : '')
					})}>Delete</button
			>
			<!-- Disable button when nothing is changed -->
			<button class="btn variant-filled-primary" formaction="?/update">Apply</button>
		</div>

		<button class="hidden" formaction="?/delete" bind:this={delete_button} />
	</section>
</form>

<style>
	input[readonly] {
		cursor: unset !important;
	}
</style>
