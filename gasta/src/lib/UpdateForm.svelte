<script lang="ts" generics="K extends StateTypes">
	import lodash from 'lodash';
	import { goto, invalidateAll } from '$app/navigation';
	import type { DisplayState, ScheduleState, StateMap, StateOf, StateTypes } from '../app';
	import * as Card from '#lib/components/ui/card/index.js';
	import { Button, buttonVariants } from './components/ui/button';
	import * as AlertDialog from '#lib/components/ui/alert-dialog/index.js';
	import type { Snippet } from 'svelte';
	import { Badge } from './components/ui/badge';
	import { CalendarClock, Monitor } from '@lucide/svelte';
	import { redirect } from '@sveltejs/kit';

	type ItemOf<T extends StateTypes = StateTypes> = {
		[P in T]: { type: P; content: StateMap[P] };
	}[T];

	type DisplayType = ItemOf<'Display'>;
	type ScheduleType = ItemOf<'Schedule'>;
	type PlaylistType = ItemOf<'Playlist'>;

	type Dependant = Exclude<ItemOf, PlaylistType>;

	let {
		uuid,
		type,
		item = $bindable(),
		update_enabled = true,
		dependant_state = null,
		children,
		onsubmit,
		ondelete
	}: {
		uuid: string;
		type: StateOf<K>;
		item?: StateMap[K];
		/** If update is allowed to be activated */
		update_enabled?: boolean;
		/** Map with entries where current type could have an item depending on it; for example a Schedule that may depend on the current Playlist */
		dependant_state?: { displays: DisplayState; schedules: ScheduleState } | null;
		children: Snippet<[StateMap[K]]>;
		onsubmit: (item: StateMap[K]) => Promise<void>;
		ondelete: (item: StateMap[K]) => Promise<void>;
	} = $props();

	const original = $derived(type?.content.get(uuid));
	// State is cloned from last committed value for
	// changes to live independently from database
	$effect(() => {
		item = structuredClone(original);
	});

	// TODO: Change type to object with two fields: schedules and displays?
	let dependents: Dependant[] | null = $derived.by(() => {
		if (!dependant_state || !uuid) return null;
		if (type?.type === 'Playlist') {
			return [
				...dependant_state.displays.content
					.values()
					.filter((v) => v.display_material.type === 'playlist' && v.display_material.uuid === uuid)
					.map((content) => ({ type: 'Display', content }) satisfies DisplayType),
				...dependant_state.schedules.content
					.values()
					.filter((v) => v.playlist === uuid || v.scheduled?.some((i) => i.playlist === uuid))
					.map((content) => ({ type: 'Schedule', content }) satisfies ScheduleType)
			];
		} else if (type?.type === 'Schedule') {
			return [
				...dependant_state.displays.content
					.values()
					.filter((v) => v.display_material.type === 'schedule' && v.display_material.uuid === uuid)
					.map((content) => ({ type: 'Display', content }) satisfies DisplayType)
			];
		}
		return null;
	});

	let disabled = $derived(!update_enabled || lodash.isEqual(item, original));
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
		class="w-full max-w-7xl"
		onsubmit={async (e) => {
			e.preventDefault();
			if (item) {
				await onsubmit(item);
				await invalidateAll();
			} else {
				console.error('`item` was not defined');
			}
		}}
	>
		<div class="w-full max-w-7xl">
			{@render children?.(item)}
			<div class="flex w-full gap-2 mt-8">
				<Button {disabled} type="submit">Update</Button>

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
								onclick={async () => {
									if (item) {
										await ondelete(item);
										await invalidateAll();
										goto(`/${type.type.toLowerCase()}`);
									}
								}}
							>
								Delete
							</AlertDialog.Action>
						</AlertDialog.Footer>
					</AlertDialog.Content>
				</AlertDialog.Root>
			</div>
		</div>
	</form>
{/if}
