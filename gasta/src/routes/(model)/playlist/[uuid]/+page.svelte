<script lang="ts">
	import type { PageData } from './$types';
	import UpdateForm from '#lib/UpdateForm.svelte';
	import type { ReadPlaylist, PlaylistItem } from '#lib/server/sasta_client/index.ts';
	import Label from '#lib/components/ui/label/label.svelte';
	import Input from '#lib/components/ui/input/input.svelte';
	import { Button, buttonVariants } from '#lib/components/ui/button/index.js';
	import { PlusIcon, EllipsisVertical, Paperclip } from '@lucide/svelte';
	import { capitalize, cn } from '#lib/utils.js';
	import { Textarea } from '#lib/components/ui/textarea/index.js';
	import * as Select from '#lib/components/ui/select/index.js';
	import * as DropdownMenu from '#lib/components/ui/dropdown-menu/index.js';
	import * as AlertDialog from '#lib/components/ui/alert-dialog/index.js';
	import DndTable, {
		renderSnippet,
		type ColumnDef
	} from '#lib/components/ui/dnd-table/DndTable.svelte';
	import EditDrawer from './EditDrawer.svelte';
	import FilePicker from '#lib/components/file-picker/FilePicker.svelte';
	import * as InputGroup from '#lib/components/ui/input-group/index.js';
	import { updatePlaylist, deletePlaylist } from '../playlists.remote';
	import { toast } from 'svelte-sonner';
	import { backendProvider, fileUploadSnippet } from '../../../files/+page.svelte';

	let { data }: { data: PageData } = $props();

	let playlist: ReadPlaylist | undefined = $state(undefined);

	const add_item = () => {
		if (!playlist) return;

		const id = crypto.randomUUID();
		playlist.items.push({
			type: 'WEBSITE',
			id,
			settings: { duration: 60, url: '' }
		});
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
				{item.type !== 'PDF' ? capitalize(item.type) : item.type}
			</span>
		</Select.Trigger>
		<Select.Content>
			{#each ['WEBSITE', 'IMAGE', 'PDF', 'TEXT'] as t}
				<Select.Item value={t}>{t !== 'PDF' ? capitalize(t) : t}</Select.Item>
			{/each}
		</Select.Content>
	</Select.Root>
{/snippet}

{#snippet DataTableValue({ item }: { item: PlaylistItem })}
	{@const base =
		'hover:bg-input/30 focus-visible:bg-background dark:hover:bg-input/30 dark:focus-visible:bg-input/30 border-transparent bg-transparent shadow-none focus-visible:border dark:bg-transparent'}
	<Label class="sr-only">Value</Label>
	{#if item.type == 'WEBSITE'}
		<InputGroup.Root>
			<InputGroup.Input class={cn(base, 'w-full')} bind:value={item.settings.url} />
			<InputGroup.Addon align="inline-end">
				<FilePicker
					onSelected={(selected) => (item.settings.url = `ASTA:/${selected.id}`)}
					api={backendProvider}
					uploadFormSnippet={fileUploadSnippet}
				>
					<InputGroup.Button variant="secondary"><Paperclip /></InputGroup.Button>
				</FilePicker>
			</InputGroup.Addon>
		</InputGroup.Root>
	{:else if item.type == 'IMAGE'}
		<InputGroup.Root>
			<InputGroup.Input class={cn(base, 'w-full')} bind:value={item.settings.src} />
			<InputGroup.Addon align="inline-end">
				<FilePicker
					onSelected={(selected) => (item.settings.src = `ASTA:/${selected.id}`)}
					api={backendProvider}
					uploadFormSnippet={fileUploadSnippet}
					defaultFilterFileType="image"
				>
					<InputGroup.Button variant="secondary"><Paperclip /></InputGroup.Button>
				</FilePicker>
			</InputGroup.Addon>
		</InputGroup.Root>
	{:else if item.type == 'PDF'}
		<InputGroup.Root>
			<InputGroup.Input class={cn(base, 'w-full')} bind:value={item.settings.path} />
			<InputGroup.Addon align="inline-end">
				<FilePicker
					onSelected={(selected) => (item.settings.path = `ASTA:/${selected.id}`)}
					api={backendProvider}
					uploadFormSnippet={fileUploadSnippet}
					defaultFilterFileType="pdf"
				>
					<InputGroup.Button variant="secondary"><Paperclip /></InputGroup.Button>
				</FilePicker>
			</InputGroup.Addon>
		</InputGroup.Root>
	{:else if item.type == 'TEXT'}
		<Textarea class={base} bind:value={item.settings.text} />
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
						playlist.items = playlist.items.filter((i: any) => i.id !== item.id);

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
	type={data.playlist}
	dependant_state={{ schedules: data.schedule, displays: data.display }}
	uuid={data.uuid}
	bind:item={playlist}
	onsubmit={async (item) => {
		try {
			await updatePlaylist({
				uuid: item.uuid,
				name: item.name,
				items: item.items
			});
			toast.success('Playlist Updated');
		} catch (error: any) {
			console.error(error);
			toast.error(error.body?.message || 'Failed to update playlist');
		}
	}}
	ondelete={async (item) => {
		try {
			await deletePlaylist({ uuid: item.uuid });
			toast.success('Playlist Deleted');
		} catch (e: any) {
			console.error(e);
			toast.error(e.body?.message || 'Failed to delete playlist');
		}
	}}
>
	{#snippet children(playlist)}
		<div class="grid gap-2 mb-5">
			<Label>Name</Label>
			<Input bind:value={playlist.name} required class="input" placeholder="Name must be unique" />
		</div>

		<div class="flex items-center justify-between w-full my-5">
			<h3>Playlist Items</h3>

			<div class="flex justify-end items-center w-1/2">
				<Button variant="outline" size="sm" type="button" onclick={add_item}>
					<PlusIcon />
					<span class="hidden lg:inline">Add Playlist Item</span>
				</Button>
			</div>
		</div>

		{#if playlist.items}
			<EditDrawer bind:open={editorOpen} bind:item={editorItem} />

			<DndTable bind:data={playlist.items} {columns} emptyMessage={'No Playlist Items added'} />
		{/if}
	{/snippet}
</UpdateForm>
