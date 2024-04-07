<script lang="ts">
	import { form_action } from '$lib/form_action';
	import { enhance } from '$app/forms';
	import lodash from 'lodash';
	import type { Playlist } from '$lib/api_bindings/read/Playlist';
	import type { Display } from '$lib/api_bindings/read/Display';
	import type { Schedule } from '$lib/api_bindings/read/Schedule';
	import { getModalStore } from '@skeletonlabs/skeleton';
	import type { State } from '../app';

	export let uuid: string;
	export let type: State;
	export let item: Display;

	$: map = type.content;
	// State is cloned from last committed value for
	// changes to live independently from database
	$: item = structuredClone(map.get(uuid));

	const modalStore = getModalStore();

	let delete_button: HTMLButtonElement;
</script>

<form
	class="card m-4"
	method="POST"
	use:enhance={({ formData }) => {
		// Ignore how forms work and send a stringified JSON of state to server route
		// A clear function on formData would have simplified things...
		[...formData.keys()].forEach((k) => formData.delete(k));

		const remove_keys = ['uuid'];

		formData.append(
			'data',
			// Destruct, filter and recreate object
			JSON.stringify(
				Object.entries(item)
					.filter(([key, value]) => !remove_keys.includes(key))
					.reduce((prev, [k, v]) => Object.assign(prev, { [k]: v }), {})
			)
		);

		return form_action;
	}}
>
	<section class="p-4">
		<slot />

		<div class="flex w-full justify-center gap-4 mt-5">
			<button
				type="button"
				class="btn variant-ringed-error"
				on:click={() =>
					modalStore.trigger({
						type: 'confirm',
						title: `Delete '${item.name}'?`,
						body: `Are your sure you want to delete ${type.type} '${item.name}'?`,
						response: (r) => (r ? delete_button.click() : '')
					})}>Delete</button
			>

			<button
				class="btn variant-filled-primary"
				disabled={lodash.isEqual(item, map.get(uuid))}
				formaction="?/update">Apply</button
			>
		</div>

		<button class="hidden" formaction="?/delete" bind:this={delete_button} />
	</section>
</form>
