<script lang="ts">
	import TypePicker from '#lib/TypePicker.svelte';
	import UpdateForm from '#lib/UpdateForm.svelte';
	import type { PageData } from './$types';
	import { Label } from '#lib/components/ui/label/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import * as InputGroup from '#lib/components/ui/input-group/index.js';
	import { CheckIcon, CopyIcon } from '@lucide/svelte';
	import { toast } from 'svelte-sonner';
	import * as Select from '#lib/components/ui/select/index.js';
	import { updateDisplay, deleteDisplay } from '../displays.remote';

	let { data }: { data: PageData } = $props();

	let other_uuid = $state('');

	let copied = $state(false);
</script>

<UpdateForm
	type={data.display}
	uuid={data.uuid}
	onsubmit={async (item) => {
		try {
			await updateDisplay({
				uuid: item.uuid,
				name: item.name,
				display_material: item.display_material
			});
			toast.success('Display Updated');
		} catch (error: any) {
			console.error(error);
			toast.error(error.body?.message || 'Failed to update display');
		}
	}}
	ondelete={async (item) => {
		try {
			await deleteDisplay({ uuid: item.uuid });
			toast.success('Display Deleted');
		} catch (e: any) {
			console.error(e);
			toast.error(e.body?.message || 'Failed to delete display');
		}
	}}
>
	{#snippet children(item)}
		<div class="grid gap-2 mb-4">
			<Label>Uuid</Label>
			<InputGroup.Root>
				<InputGroup.Input bind:value={item.uuid} class="text-zinc-500" readonly />
				<InputGroup.Addon align="inline-end">
					<InputGroup.Button
						aria-label="Copy"
						title="Copy"
						size="icon-xs"
						onclick={async () => {
							try {
								await navigator.clipboard.writeText(item.uuid);
								copied = true;
								setTimeout(() => (copied = false), 2000);
							} catch (err) {
								toast('Could not copy Uuid, ' + err);
							}
						}}
					>
						{#if copied}
							<CheckIcon />
						{:else}
							<CopyIcon />
						{/if}
					</InputGroup.Button>
				</InputGroup.Addon>
			</InputGroup.Root>
		</div>

		<div class="grid gap-2 mb-4">
			<Label>Name</Label>
			<Input bind:value={item.name} required class="input" placeholder="Name must be unique" />
		</div>

		<h3 class="mb-2">Display Material</h3>

		<div class="flex md:items-center w-full gap-4 sm:flex-row flex-col">
			<Select.Root
				type="single"
				bind:value={
					() => item.display_material.type,
					(v) => {
						if (v) {
							if (item.display_material.type !== v) {
								const temp = item.display_material.uuid;
								item.display_material.uuid = other_uuid;
								other_uuid = temp;
							}
							item.display_material.type = v;
						}
					}
				}
			>
				{@const type = item.display_material.type}
				<Select.Trigger class="w-32">
					{type.charAt(0).toUpperCase()}{type.substring(1)}
				</Select.Trigger>
				<Select.Content>
					<Select.Label>Types</Select.Label>
					<Select.Item value="schedule">Schedule</Select.Item>
					<Select.Item value="playlist">Playlist</Select.Item>
				</Select.Content>
			</Select.Root>
			<div class="w-full">
				<TypePicker
					label={false}
					types={item.display_material.type === 'schedule' ? data.schedule : data.playlist}
					bind:chosen_type={item.display_material.uuid}
				/>
			</div>
		</div>
	{/snippet}
</UpdateForm>
