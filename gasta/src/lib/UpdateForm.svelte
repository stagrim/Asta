<script lang="ts">
	import { Icon } from 'svelte-awesome';
	import calendar from 'svelte-awesome/icons/calendar';
	import tv from 'svelte-awesome/icons/tv';

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
	export let item: Display | Schedule | Playlist;
	export let update_enabled: boolean = true;
	/** Map with entries where current type could have an item depending on it; for example a Schedule that may depend on the current Playlist */
	export let dependant_state:
		| Exclude<State, { type: 'Playlist'; content: Map<string, Playlist> }>
		| undefined;

	$: map = type.content;
	// State is cloned from last committed value for
	// changes to live independently from database
	$: item = structuredClone(map.get(uuid)!);

	const modalStore = getModalStore();

	let delete_button: HTMLButtonElement;

	const filter_state = (
		uuid: string
	): ((predicate: [string, Display] | [string, Schedule]) => boolean) => {
		if (dependant_state?.type === 'Display') {
			return ([_k, v]) => (v as Display).schedule === uuid;
		} else {
			return ([_k, v]) =>
				(v as Schedule).playlist === uuid ||
				(v as Schedule).scheduled!.some((i) => i.playlist === uuid);
		}
	};

	$: dependents = dependant_state
		? [...dependant_state.content.entries()].filter(filter_state(uuid))
		: null;
</script>

<!--  Give fields linked to id of uuid to highlight how they are dependant on link -->
{#if dependant_state && dependents}
	<div class="card m-4 p-4">
		<div class="flex overflow-scroll hide-scrollbar gap-2">
			{#if dependents.length < 1}
				<i class="text-surface-300">No dependents... :(</i>
			{/if}
			{#each dependents as [uuid, { name }] (uuid)}
				<a href={`/${dependant_state.type.toLowerCase()}/${uuid}`}>
					<span class="chip variant-ghost-primary gap-1"
						><Icon data={dependant_state.type === 'Display' ? tv : calendar}></Icon> &nbsp;{name}</span
					>
				</a>
			{/each}
		</div>
	</div>
{/if}

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
				disabled={lodash.isEqual(item, map.get(uuid)) || !update_enabled}
				formaction="?/update">Apply</button
			>
		</div>

		<!-- svelte-ignore a11y_consider_explicit_label -->
		<button class="hidden" formaction="?/delete" bind:this={delete_button}></button>
	</section>
</form>
