<script lang="ts">
	import copy from 'svelte-awesome/icons/copy';

	import { page } from '$app/stores';
	import TypePicker from '../../../lib/TypePicker.svelte';
	import UpdateForm from '$lib/UpdateForm.svelte';
	import { toastStore } from '$lib/stores';
	import type { PageData } from './$types';
	import { Icon } from 'svelte-awesome';

	export let data: PageData;

	$: uuid = $page.params.uuid;

	let item;
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<!-- svelte-ignore a11y-no-static-element-interactions -->
<!-- svelte-ignore a11y-missing-attribute -->
<UpdateForm bind:type={data.display} bind:uuid bind:item>
	{#if item}
		<span class="mb-5">
			<span>Uuid</span>
			<div
				class="input-group input-group-divider grid-cols-[1fr_auto] cursor-pointer"
				on:click={async (_) => {
					try {
						await navigator.clipboard.writeText(item.uuid);
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
				<input class="input text-surface-400" type="text" readonly value={item.uuid} />
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
				bind:value={item.name}
			/>
		</label>

		<TypePicker name="schedule" bind:chosen_type={item.schedule} types={data.schedule} />
	{/if}
</UpdateForm>

<style>
	input[readonly] {
		cursor: unset !important;
	}
</style>
