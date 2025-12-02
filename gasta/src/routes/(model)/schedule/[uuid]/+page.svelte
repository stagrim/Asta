<script lang="ts">
	import type { PageData } from './$types';
	import TypePicker from '$lib/TypePicker.svelte';
	import type { Schedule } from '$lib/api_bindings/read/Schedule';
	import UpdateForm from '$lib/UpdateForm.svelte';
	import CronField from './CronField.svelte';
	import { onMount } from 'svelte';
	import DndTable, {
		renderSnippet,
		type ColumnDef
	} from '$lib/components/ui/dnd-table/DndTable.svelte';
	import type { ScheduledPlaylistInput } from '$lib/api_bindings/update/ScheduledPlaylistInput';
	import { Label } from '$lib/components/ui/label';
	import { Input } from '$lib/components/ui/input';
	import { Button } from '$lib/components/ui/button';
	import { EllipsisVertical, ListVideo, PlusIcon, Server } from '@lucide/svelte';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import { watch } from 'runed';
	import cron from 'cron-validate';
	import { Badge } from '$lib/components/ui/badge';
	import { cn } from '$lib/utils';
	import * as Card from '$lib/components/ui/card';

	let { data }: { data: PageData } = $props();

	let schedule: Schedule | undefined = $state(undefined);

	const add_item = () => {
		if (schedule) {
			schedule.scheduled = [
				...(schedule.scheduled ?? []),
				{ id: crypto.randomUUID(), playlist: '', start: '', end: '' }
			];
		}
	};

	let update_enabled = $derived.by(() => {
		if (schedule?.scheduled) {
			return schedule.scheduled
				.flatMap((s) => [s.start, s.end])
				.every((c) =>
					cron(c.trim(), {
						override: {
							useSeconds: true,
							useYears: true,
							useAliases: true
						}
					}).isValid()
				);
		}
		return false;
	});
	let schedule_info = $state(data.schedule_info);

	let timeout_handle: {
		readonly promise: Promise<unknown>;
		cancel(): void;
	};
	const clear_timeout = () => (timeout_handle ? timeout_handle.cancel() : null);
	// Clear current timeout if server load function has changed schedule_info on navigation
	watch(
		() => data.uuid,
		() => {
			clear_timeout();
		}
	);

	/**
	 * Cancelable delay object
	 * @param delay delay in milliseconds
	 */
	const later = (delay: number) => {
		let timer: ReturnType<typeof setTimeout> | null = null;
		let reject: (reason?: any) => void;
		const promise = new Promise((resolve, _reject) => {
			reject = _reject;
			timer = setTimeout(resolve, delay);
		});
		return {
			/** Returns the promise to be awaited */
			get promise() {
				return promise;
			},
			/** Clears the delay and throws an error */
			cancel() {
				if (timer) {
					clearTimeout(timer);
					timer = null;
					reject();
					reject = () => {};
				}
			}
		};
	};

	const updateLoop = async () => {
		const MAX_TIMEOUT = 2147483647;

		while (true) {
			let sleep: number = (schedule_info.next?.in_ms as any as number) ?? 99999999;
			if (sleep > MAX_TIMEOUT) {
				// console.warn(`Sleep duration ${sleep}ms exceeds max. Capping at ${MAX_TIMEOUT}ms.`);
				sleep = MAX_TIMEOUT;
			}

			timeout_handle = later(sleep);
			try {
				await timeout_handle.promise;
			} catch {}

			try {
				schedule_info = await (await fetch(`/schedule/${data.uuid}`)).json();
			} catch (e) {
				console.error(e);
			}
		}
	};

	onMount(() => {
		updateLoop();
	});

	let columns: ColumnDef<ScheduledPlaylistInput & { id: string }>[] = [
		{
			id: 'active',
			label: 'Active',
			render: (item) => renderSnippet(DataTableActive, { item })
		},
		{
			id: 'playlist',
			label: 'Playlist',
			render: (item) => renderSnippet(DataTablePlaylist, { item }),
			class: 'w-1/3'
		},
		{
			id: 'start',
			label: 'Start',
			render: (item) => renderSnippet(DataTableStart, { item }),
			class: 'w-1/3 min-w-[250px]'
		},
		{
			id: 'end',
			label: 'End',
			render: (item) => renderSnippet(DataTableEnd, { item }),
			class: 'w-1/3 min-w-[250px]'
		},
		{
			id: 'actions',
			label: '',
			render: (item) => renderSnippet(DataTableActions, { item })
		}
	];

	let bind_update_enabled = $state(false);
</script>

{#snippet DataTableActive({ item }: { item: { id: string; playlist: string } })}
	<Badge
		class={[
			'rounded-md h-5 w-5 align-middle transition-colors',
			!bind_update_enabled && schedule_info.current === item.playlist && 'bg-primary',
			!bind_update_enabled && schedule_info?.next?.playlist === item.playlist && 'bg-primary/20'
		]}
		variant="outline"
	></Badge>
{/snippet}

{#snippet DataTablePlaylist({ item }: { item: ScheduledPlaylistInput })}
	<Label class="sr-only">Playlist</Label>
	<TypePicker
		label={false}
		types={data.playlist}
		name="playlist"
		bind:chosen_type={item.playlist}
	/>
{/snippet}

{#snippet DataTableStart({ item }: { item: ScheduledPlaylistInput })}
	<CronField bind:value={item.start} />
{/snippet}

{#snippet DataTableEnd({ item }: { item: ScheduledPlaylistInput })}
	<CronField bind:value={item.end} />
{/snippet}

{#snippet DataTableActions({ item }: { item: ScheduledPlaylistInput })}
	<DropdownMenu.Root>
		<DropdownMenu.Trigger class="data-[state=open]:bg-muted text-muted-foreground flex size-8">
			{#snippet child({ props })}
				<Button variant="ghost" size="icon" {...props}>
					<EllipsisVertical />
					<span class="sr-only">Open menu</span>
				</Button>
			{/snippet}
		</DropdownMenu.Trigger>
		<DropdownMenu.Content align="end" class="w-32">
			<DropdownMenu.Item disabled>Edit</DropdownMenu.Item>
			<DropdownMenu.Separator />
			<DropdownMenu.Item
				variant="destructive"
				onclick={() => {
					if (schedule) {
						schedule.scheduled = schedule.scheduled?.filter((i) => i.id !== item.id) ?? null;
					}
				}}>Delete</DropdownMenu.Item
			>
		</DropdownMenu.Content>
	</DropdownMenu.Root>
{/snippet}

<UpdateForm
	bind:type={data.schedule}
	dependant_state={{ schedules: data.schedule, displays: data.display }}
	bind:uuid={data.uuid}
	bind:item={schedule}
	bind:update_enabled
	bind:bind_update_enabled
>
	{#if schedule}
		<div class="grid gap-2 mb-5">
			<Label>Name</Label>
			<Input
				required
				name="name"
				class="input"
				type="text"
				placeholder="Name must be unique"
				bind:value={schedule.name}
			/>
		</div>

		<div class="flex items-center justify-between w-full my-5">
			<h3>Scheduled Playlists</h3>

			<div class="flex justify-end items-center w-1/2">
				<Button variant="outline" size="sm" onclick={add_item}>
					<PlusIcon />
					<span class="hidden lg:inline">Add Scheduled Playlist</span>
				</Button>
			</div>
		</div>
		<!-- Must cast to any because of a bad hack that gives all a random id in the +layout.server.ts file -->
		<!-- TODO: If type hacking continues, then make a union type globally. Otherwise fix server side -->
		<DndTable
			bind:data={
				() => (schedule?.scheduled ?? []) as any,
				(v) => {
					if (schedule) {
						schedule.scheduled = v;
					}
				}
			}
			{columns}
			emptyMessage="No Scheduled Playlists added"
		>
			{#snippet preRows()}
				{@const baseClasses =
					'hover:[&,&>svelte-css-wrapper]:[&>th,td]:bg-muted/20 data-[state=selected]:bg-muted bg-background/20 border-b transition-colors'}
				{#if schedule}
					<tr class={cn(baseClasses)}>
						<td class={cn(baseClasses, 'px-5')}> </td>

						<td class={cn(baseClasses, 'p-2')}>
							{@render DataTableActive({ item: { id: '', playlist: schedule.playlist } })}
						</td>
						<td class={cn(baseClasses, 'w-1/3 p-2')}>
							<Label class="sr-only">Playlist</Label>
							<TypePicker
								label={false}
								types={data.playlist}
								name="playlist"
								bind:chosen_type={schedule.playlist}
							/>
						</td>
						<td class={cn(baseClasses, 'italic p-2 text-foreground/60')}> Fallback </td>
						<td class={cn(baseClasses, 'italic p-2 text-foreground/60')} colspan="20">
							Fallback
						</td>
					</tr>
				{/if}
			{/snippet}
		</DndTable>

		<div class="mt-4">
			{#if schedule_info.next}
				{@const next_playlist = data.playlist.content.get(schedule_info.next.playlist)}
				{@const change_date = new Date(Date.now() + (schedule_info.next.in_ms as any as number))}
				{#if next_playlist}
					Will change to
					<a href={`/playlist/${next_playlist.uuid}`}>
						<Badge>
							<ListVideo />
							&nbsp;{next_playlist.name}
						</Badge>
					</a>
					at {change_date.toLocaleString()}
				{:else}
					Playlist {schedule_info.next.playlist} does not exist :(
				{/if}
			{:else}
				<i class="text-foreground/60">No playlist change is scheduled</i>
			{/if}
		</div>
	{/if}
</UpdateForm>
