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
	import type { DisplayState, ScheduleState, State } from '../app';

	export let uuid: string;
	export let type: State;
	export let item: Display | Schedule | Playlist | undefined;
	export let update_enabled: boolean = true;

	type DisplayType = { type: 'Display'; content: Display };
	type ScheduleType = { type: 'Schedule'; content: Schedule };
	type PlaylistType = { type: 'Playlist'; content: Playlist };

	type Types = DisplayType | ScheduleType | PlaylistType;
	type Dependant = Exclude<Types, PlaylistType>;
	type HasDependencies = Exclude<Types, DisplayType>;

	/** Map with entries where current type could have an item depending on it; for example a Schedule that may depend on the current Playlist */
	export let dependant_state: { displays: DisplayState; schedules: ScheduleState } | null = null;

	$: map = type.content;
	// State is cloned from last committed value for
	// changes to live independently from database
	$: item = structuredClone(map.get(uuid));

	const modalStore = getModalStore();

	let delete_button: HTMLButtonElement;

	const test = ({ content, type }: HasDependencies): Dependant[] => {
		if (dependant_state) {
			if (type === 'Playlist') {
				return [
					...dependant_state.displays.content
						.entries()
						.filter(
							([, v]) =>
								v.display_material.type === 'playlist' && v.display_material.uuid === content.uuid
						)
						.map(([, content]) => ({ type: 'Display', content }) satisfies DisplayType),
					...dependant_state.schedules.content
						.entries()
						.filter(
							([, v]) =>
								v.playlist === content.uuid || v.scheduled?.some((i) => i.playlist === content.uuid)
						)
						.map(([, content]) => ({ type: 'Schedule', content }) satisfies ScheduleType)
				];
			} else if (type === 'Schedule') {
				return [
					...dependant_state.displays.content
						.entries()
						.filter(
							([, v]) =>
								v.display_material.type === 'schedule' && v.display_material.uuid === content.uuid
						)
						.map(([, content]) => ({ type: 'Display', content }) satisfies DisplayType)
				];
			}
		}
		return [];
	};

	// TODO: Change type to object with two fields: schedules and displays?
	let dependents: Dependant[] | null = null;
	$: {
		if (type.type === 'Schedule') {
			dependents = test({ type: 'Schedule', content: item as Schedule });
		} else if (type.type === 'Playlist') {
			dependents = test({ type: 'Playlist', content: item as Playlist });
		}
	}
</script>

<!--  Give fields linked to id of uuid to highlight how they are dependant on link -->
{#if dependant_state && dependents}
	<div class="card m-4 p-4">
		<div class="flex overflow-scroll hide-scrollbar gap-2">
			{#if dependents.length < 1}
				<i class="text-surface-300">No dependents... :(</i>
			{/if}
			{#each dependents as { content: { name, uuid }, type } (name + uuid)}
				<a href={`/${type.toLowerCase()}/${uuid}`}>
					<span class="chip variant-ghost-primary gap-1"
						><Icon data={type === 'Display' ? tv : calendar}></Icon> &nbsp;{name}</span
					>
				</a>
			{/each}
		</div>
	</div>
{/if}

{#if item}
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
{/if}
