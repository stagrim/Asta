<script lang="ts">
	import type { PageData } from './$types';
	import TypePicker from '#lib/TypePicker.svelte';
	import UpdateForm from '#lib/UpdateForm.svelte';
	import CronField from './CronField.svelte';
	import { onMount } from 'svelte';
	import DndTable, {
		renderSnippet,
		type ColumnDef
	} from '#lib/components/ui/dnd-table/DndTable.svelte';
	import { Label } from '#lib/components/ui/label/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import { Button, buttonVariants } from '#lib/components/ui/button/index.js';
	import { EllipsisVertical, ListVideo, PlusIcon, Server } from '@lucide/svelte';
	import * as DropdownMenu from '#lib/components/ui/dropdown-menu/index.js';
	import * as AlertDialog from '#lib/components/ui/alert-dialog/index.js';
	import { watch } from 'runed';
	import cronValidate from 'cron-validate';
	const cron = typeof cronValidate === 'function' ? cronValidate : (cronValidate as any).default;
	import { Badge } from '#lib/components/ui/badge/index.js';
	import { cn } from '#lib/utils.js';
	import type {
		ReadSchedule,
		ScheduledPlaylistInput,
		ScheduleInfo
	} from '#lib/server/sasta_client/index.ts';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import { updateSchedule, deleteSchedule, scheduleInfo } from '../schedules.remote';
	import { toast } from 'svelte-sonner';

	let { data }: { data: PageData } = $props();

	let schedule: ReadSchedule | undefined = $state(undefined);

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
				.flatMap((s: any) => [s.start, s.end])
				.every((c: any) =>
					cron(c.trim(), {
						override: {
							useSeconds: true,
							useYears: true,
							useAliases: true
						}
					}).isValid()
				);
		}
		return true;
	});

	let schedule_info = $state<Promise<ScheduleInfo | undefined>>();

	let timeout_handle: {
		readonly promise: Promise<unknown>;
		cancel(): void;
	};
	const clear_timeout = () => (timeout_handle ? timeout_handle.cancel() : undefined);
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

	const updateLoop = async (isCancelled: () => boolean) => {
		const MAX_TIMEOUT = 2147483647;

		while (!isCancelled()) {
			let sleep: number = 0;

			const awaited_schedule_info = await schedule_info;
			if (isCancelled()) break;
			if (schedule_info && awaited_schedule_info) {
				sleep = Math.max(
					0,
					new Date(awaited_schedule_info.next?.moment ?? '2038-01-19T03:14:07Z').valueOf() -
						new Date().valueOf()
				);
			}

			if (sleep > MAX_TIMEOUT) {
				console.warn(`Sleep duration ${sleep}ms exceeds max. Capping at ${MAX_TIMEOUT}ms.`);
				sleep = MAX_TIMEOUT;
			}

			timeout_handle = later(sleep);
			try {
				await timeout_handle.promise;
			} catch {}

			if (isCancelled()) break;

			try {
				schedule_info = scheduleInfo({ uuid: data.uuid });
			} catch (e) {
				console.error(e);
			}
		}
	};

	onMount(() => {
		let cancelled = false;
		updateLoop(() => cancelled);
		return () => {
			cancelled = true;
			clear_timeout();
		};
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

	// TODO: fix this
	let bind_update_enabled = $state(false);
</script>

{#snippet DataTableActive({ item }: { item: { id: string; playlist: string } })}
	{#await schedule_info}
		<Skeleton class="h-5 w-5" />
	{:then schedule_info}
		<Badge
			class={[
				'rounded-md h-5 w-5 align-middle transition-colors',
				!bind_update_enabled && schedule_info?.current === item.playlist && 'bg-primary',
				!bind_update_enabled && schedule_info?.next?.playlist === item.playlist && 'bg-primary/20'
			]}
			variant="outline"
		></Badge>
	{/await}
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
	type={data.schedule}
	dependant_state={{ schedules: data.schedule, displays: data.display }}
	uuid={data.uuid}
	bind:item={schedule}
	{update_enabled}
	onsubmit={async (item) => {
		try {
			await updateSchedule({
				uuid: item.uuid,
				name: item.name,
				playlist: item.playlist,
				scheduled: item.scheduled
			});
			toast.success('Schedule Updated');
		} catch (error: any) {
			console.error(error);
			toast.error(error.body?.message || 'Failed to update schedule');
		}
	}}
	ondelete={async (item) => {
		try {
			await deleteSchedule({ uuid: item.uuid });
			toast.success('Schedule Deleted');
		} catch (e: any) {
			console.error(e);
			toast.error(e.body?.message || 'Failed to delete schedule');
		}
	}}
>
	{#snippet children(schedule)}
		<div class="grid gap-2 mb-5">
			<Label>Name</Label>
			<Input bind:value={schedule.name} required class="input" placeholder="Name must be unique" />
		</div>

		<div class="flex items-center justify-between w-full my-5">
			<h3>Scheduled Playlists</h3>

			<div class="flex justify-end items-center w-1/2">
				<Button variant="outline" size="sm" type="button" onclick={add_item}>
					<PlusIcon />
					<span class="hidden lg:inline">Add Scheduled Playlist</span>
				</Button>
			</div>
		</div>
		<!-- Must cast to any because of a bad hack that gives all a random id in the +layout.server.ts file -->
		<!-- TODO: If type hacking continues, then make a union type globally. Otherwise fix server side -->
		<DndTable
			bind:data={
				() => schedule.scheduled ?? [],
				(v) => {
					schedule.scheduled = v;
				}
			}
			{columns}
			emptyMessage="No Scheduled Playlists added"
		>
			{#snippet preRows()}
				{@const baseClasses =
					'hover:[&,&>svelte-css-wrapper]:[&>th,td]:bg-muted/20 data-[state=selected]:bg-muted bg-background/20 border-b transition-colors'}
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
					<td class={cn(baseClasses, 'italic p-2 text-foreground/60')} colspan="20"> Fallback </td>
				</tr>
			{/snippet}
		</DndTable>

		<div class="mt-4 h-5">
			{#await schedule_info}
				<Skeleton class="h-4 w-62.5" />
			{:then schedule_info}
				{#if schedule_info?.next}
					{@const next_playlist = data.playlist.content.get(schedule_info.next.playlist)}
					{@const change_date = new Date(schedule_info.next.moment)}
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
			{/await}
		</div>
	{/snippet}
</UpdateForm>
