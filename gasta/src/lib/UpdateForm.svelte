<script lang="ts">
	import { form_action } from '$lib/form_action';
	import { enhance } from '$app/forms';
	import lodash from 'lodash';
	import type { Playlist } from '$lib/api_bindings/read/Playlist';
	import type { Display } from '$lib/api_bindings/read/Display';
	import type { Schedule } from '$lib/api_bindings/read/Schedule';
	import type { DisplayState, ScheduleState, State } from '../app';
	import * as Card from '$lib/components/ui/card';
	import { Button, buttonVariants } from './components/ui/button';
	import * as AlertDialog from '$lib/components/ui/alert-dialog';
	import type { Snippet } from 'svelte';
	import { watch } from 'runed';
	import { Badge } from './components/ui/badge';
	import { CalendarClock, ListVideo, Monitor } from '@lucide/svelte';

	type DisplayType = { type: 'Display'; content: Display };
	type ScheduleType = { type: 'Schedule'; content: Schedule };
	type PlaylistType = { type: 'Playlist'; content: Playlist };

	type Types = DisplayType | ScheduleType | PlaylistType;
	type Dependant = Exclude<Types, PlaylistType>;
	type HasDependencies = Exclude<Types, DisplayType>;

	let {
		uuid = $bindable(),
		type = $bindable(),
		item = $bindable(undefined),
		bind_update_enabled = $bindable(),
		update_enabled = $bindable(true),
		dependant_state = null,
		children
	}: {
		uuid: string;
		type: State;
		item: Display | Schedule | Playlist | undefined;
		bind_update_enabled?: boolean;
		/** If update is allowed to be activated */
		update_enabled?: boolean;
		/** Map with entries where current type could have an item depending on it; for example a Schedule that may depend on the current Playlist */
		dependant_state?: { displays: DisplayState; schedules: ScheduleState } | null;
		children: Snippet<[]>;
	} = $props();

	const map = $derived(type?.content);
	// State is cloned from last committed value for
	// changes to live independently from database
	$effect(() => {
		if (map && uuid) {
			item = structuredClone(map.get(uuid));
		}
	});

	let delete_button: HTMLButtonElement | undefined = $state();

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
	let dependents: Dependant[] | null = $state(null);
	$effect(() => {
		if (type?.type === 'Schedule') {
			dependents = test({ type: 'Schedule', content: item as Schedule });
		} else if (type?.type === 'Playlist') {
			dependents = test({ type: 'Playlist', content: item as Playlist });
		}
	});

	watch(
		() => disabled,
		() => {
			bind_update_enabled = !disabled;
		}
	);

	let disabled = $derived((map && uuid && lodash.isEqual(item, map.get(uuid))) || !update_enabled);
</script>

<!--  Give fields linked to id of uuid to highlight how they are dependant on link -->
{#if dependant_state && dependents}
	<Card.Root class="relative w-full max-w-7xl mb-4 py-4 px-0">
		<Card.Content class="overflow-x-auto">
			<div class="flex overflow-x-auto hide-scrollbar gap-2">
				{#if !dependents.length}
					<i class="text-foreground/60">No dependents... :(</i>
				{:else}
					<p class="mr-2">In use by</p>
				{/if}
				{#each dependents as { content: { name, uuid }, type } (name + uuid)}
					<a href={`/${type.toLowerCase()}/${uuid}`}>
						<Badge>
							{#if type === 'Display'}
								<Monitor />
							{:else}
								<CalendarClock />
							{/if}
							&nbsp;{name}
						</Badge>
					</a>
				{/each}
			</div>
		</Card.Content>
	</Card.Root>
{/if}

{#if item}
	<form
		method="POST"
		class="w-full max-w-7xl"
		use:enhance={({ formData }) => {
			if (!item) {
				console.error("Cannot submit form: 'item' is undefined.");
				return () => {};
			}
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
		{@render children?.()}
		<div class="flex w-full gap-2 mt-8">
			<Button {disabled} type="submit" formaction="?/update">Update</Button>

			<AlertDialog.Root>
				<AlertDialog.Trigger type="button" class={buttonVariants({ variant: 'outline' })}>
					Delete
				</AlertDialog.Trigger>
				<AlertDialog.Content>
					<AlertDialog.Header>
						<AlertDialog.Title>Delete '{item.name}'?</AlertDialog.Title>
						<AlertDialog.Description>
							Are your sure you want to delete {type?.type} '{item.name}'?
						</AlertDialog.Description>
					</AlertDialog.Header>
					<AlertDialog.Footer>
						<AlertDialog.Cancel>Cancel</AlertDialog.Cancel>
						<AlertDialog.Action
							class={buttonVariants({ variant: 'destructive' })}
							onclick={() => delete_button?.click()}
						>
							Delete
						</AlertDialog.Action>
					</AlertDialog.Footer>
				</AlertDialog.Content>
			</AlertDialog.Root>
		</div>

		<!-- svelte-ignore a11y_consider_explicit_label -->
		<button class="hidden" formaction="?/delete" bind:this={delete_button}></button>
	</form>
{/if}
