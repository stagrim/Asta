<script lang="ts">
	import type { PageData } from './$types';
	import UpdateForm from '$lib/UpdateForm.svelte';
	import type { Playlist } from '$lib/api_bindings/read/Playlist';
	import Label from '$lib/components/ui/label/label.svelte';
	import Input from '$lib/components/ui/input/input.svelte';
	import { Button } from '$lib/components/ui/button';
	import { PlusIcon, EllipsisVertical } from '@lucide/svelte';
	import { capitalize, cn } from '$lib/utils';
	import type { PlaylistItem } from '$lib/api_bindings/update/PlaylistItem';
	import { Textarea } from '$lib/components/ui/textarea';
	import * as Select from '$lib/components/ui/select';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import DndTable, {
		renderSnippet,
		type ColumnDef
	} from '$lib/components/ui/dnd-table/DndTable.svelte';
	import EditDrawer from './EditDrawer.svelte';

	let { data }: { data: PageData } = $props();

	let playlist: Playlist | undefined = $state(undefined);

	const add_item = () => {
		if (!playlist) return;

		const id = crypto.randomUUID();
		playlist.items.push({
			type: 'WEBSITE',
			id,
			settings: { duration: 60 as unknown as bigint, url: '' }
		});
	};

	const swap_item = (a: number, b: number) => {
		if (playlist) {
			const tmp = playlist.items[a];
			playlist.items[a] = playlist.items[b];
			playlist.items[b] = tmp;
		}
	};

	let columns: ColumnDef<PlaylistItem>[] = [
		{
			id: 'type',
			label: 'Type',
			render: (item) => renderSnippet(DataTableType, { item })
		},
		{
			id: 'value',
			label: 'Value',
			render: (item) => renderSnippet(DataTableValue, { item }),
			class: 'w-full min-w-[300px]'
		},
		{
			id: 'duration',
			label: 'Duration',
			render: (item) => renderSnippet(DataTableDuration, { item })
		},
		{
			id: 'actions',
			label: '',
			render: (item) => renderSnippet(DataTableActions, { item })
		}
	];

	let editorOpen = $state(false);
	let editorItem: PlaylistItem | undefined = $state(undefined);
</script>

{#snippet DataTableType({ item }: { item: PlaylistItem })}
	<Label class="sr-only">Type</Label>
	<Select.Root type="single" bind:value={item.type}>
		<Select.Trigger
			class="w-28 **:data-[slot=select-value]:block **:data-[slot=select-value]:truncate"
			size="sm"
		>
			<span data-slot="select-value">
				{capitalize(item.type)}
			</span>
		</Select.Trigger>
		<Select.Content>
			{#each ['WEBSITE', 'IMAGE', 'TEXT'] as t}
				<Select.Item value={t}>{capitalize(t)}</Select.Item>
			{/each}
		</Select.Content>
	</Select.Root>
{/snippet}

{#snippet DataTableValue({ item }: { item: PlaylistItem })}
	{@const base =
		'hover:bg-input/30 focus-visible:bg-background dark:hover:bg-input/30 dark:focus-visible:bg-input/30 border-transparent bg-transparent shadow-none focus-visible:border dark:bg-transparent'}
	<Label class="sr-only">Value</Label>
	{#if item.type == 'WEBSITE'}
		<Input class={cn(base, 'w-full')} bind:value={item.settings.url} />
	{:else if item.type == 'TEXT'}
		<Textarea class={base} bind:value={item.settings.text} />
	{:else if item.type == 'IMAGE'}
		<Input class={cn(base, 'w-full')} bind:value={item.settings.src} />
	{/if}
{/snippet}

{#snippet DataTableDuration({ item }: { item: PlaylistItem })}
	<Label class="sr-only">Duration</Label>
	<Input
		class="no-spinner hover:bg-input/30 focus-visible:bg-background dark:hover:bg-input/30 dark:focus-visible:bg-input/30 h-8 w-16 border-transparent bg-transparent text-right shadow-none focus-visible:border dark:bg-transparent"
		type="number"
		bind:value={item.settings.duration}
	/>
{/snippet}

{#snippet DataTableActions({ item }: { item: PlaylistItem })}
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
			<DropdownMenu.Item
				onclick={() => {
					editorOpen = true;
					editorItem = item;
				}}
			>
				Edit
			</DropdownMenu.Item>
			<!-- <DropdownMenu.Item
				onclick={() => {
					if (playlist) {
						const index = playlist.items.findIndex((i) => i === item);
						console.log(item);
						const newItem = structuredClone(item);
						newItem.id = crypto.randomUUID();
						playlist.items.push(newItem);
					}
				}}>Make a copy</DropdownMenu.Item
			> -->
			<DropdownMenu.Separator />
			<DropdownMenu.Item
				onclick={() => {
					if (playlist) {
						playlist.items = playlist.items.filter((i) => i.id !== item.id);

						if (editorItem?.id === item.id) {
							editorOpen = false;
							editorItem = undefined;
						}
					}
				}}
				variant="destructive">Delete</DropdownMenu.Item
			>
		</DropdownMenu.Content>
	</DropdownMenu.Root>
{/snippet}

<UpdateForm
	bind:type={data.playlist}
	dependant_state={{ schedules: data.schedule, displays: data.display }}
	bind:uuid={data.uuid}
	bind:item={playlist}
>
	{#if playlist}
		<div class="grid gap-2 mb-5">
			<Label>Name</Label>
			<Input
				required
				name="name"
				class="input"
				type="text"
				placeholder="Name must be unique"
				bind:value={playlist.name}
			/>
		</div>

		<div class="flex items-center justify-between w-full my-5">
			<h3>Playlist Items</h3>

			<div class="flex justify-end items-center w-1/2">
				<Button variant="outline" size="sm" onclick={add_item}>
					<PlusIcon />
					<span class="hidden lg:inline">Add Playlist Item</span>
				</Button>
			</div>
		</div>

		{#if playlist.items}
			<EditDrawer bind:open={editorOpen} bind:item={editorItem} />

			<DndTable bind:data={playlist.items} {columns} emptyMessage={'No Playlist Items added'} />
		{/if}
	{/if}
</UpdateForm>
