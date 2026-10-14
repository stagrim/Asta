<script lang="ts">
	import TypePicker from '#lib/TypePicker.svelte';
	import type { PageData } from './$types';
	import Label from '#lib/components/ui/label/label.svelte';
	import Input from '#lib/components/ui/input/input.svelte';
	import * as Select from '#lib/components/ui/select/index.js';
	import Button from '#lib/components/ui/button/button.svelte';
	import { createDisplay } from './displays.remote';
	import { toast } from 'svelte-sonner';

	let { data }: { data: PageData } = $props();

	const { name, uuid, display_material } = createDisplay.fields;
	if (!display_material.type.value()) {
		display_material.type.set('playlist');
	}
</script>

<form
	class="w-full max-w-7xl"
	{...createDisplay.enhance(async ({ submit }) => {
		try {
			if (await submit()) {
				toast.success('Display Added');
			} else {
				toast.error(createDisplay.fields.allIssues()?.[0]?.message || 'Failed to add display');
			}
		} catch (error: any) {
			toast.error(error.body?.message || 'Failed to add display');
		}
	})}
>
	<div class="grid gap-2 mb-4">
		<Label>Uuid</Label>
		<Input {...uuid.as('text')} class="input" placeholder="Uuid for display (optional)" />
	</div>

	<div class="grid gap-2 mb-4">
		<Label>Name</Label>
		<Input {...name.as('text')} required class="input" placeholder="Name must be unique" />
	</div>

	<h3 class="mb-2">Display Material</h3>

	<div class="flex md:items-center w-full gap-4 sm:flex-row flex-col">
		<Select.Root type="single" {...display_material.type.as('select', 'playlist')}>
			{@const type = display_material.type.value() ?? 'playlist'}
			<Select.Trigger class="w-32">
				{type.charAt(0).toUpperCase()}{type.substring(1)}
			</Select.Trigger>
			<Select.Content>
				<Select.Label>Types</Select.Label>
				<Select.Item value="schedule">Schedule</Select.Item>
				<Select.Item value="playlist">Playlist</Select.Item>
			</Select.Content>
		</Select.Root>
		<div class="w-full justify-center">
			{#if display_material.type.value() === 'schedule'}
				<TypePicker {...display_material.uuid.as('select')} label={false} types={data.schedule} />
			{:else}
				<TypePicker {...display_material.uuid.as('select')} label={false} types={data.playlist} />
			{/if}
		</div>
	</div>

	<div class="mt-8">
		<Button type="submit">Add</Button>
	</div>
</form>
