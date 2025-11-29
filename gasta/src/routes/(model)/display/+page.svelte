<script lang="ts">
	import * as Card from '$lib/components/ui/card';
	import { enhance } from '$app/forms';
	import { form_action } from '$lib/form_action';
	import TypePicker from '$lib/TypePicker.svelte';
	import type { PageData } from './$types';
	import Label from '$lib/components/ui/label/label.svelte';
	import Input from '$lib/components/ui/input/input.svelte';
	import * as Select from '$lib/components/ui/select/index.js';
	import Button from '$lib/components/ui/button/button.svelte';

	let { data }: { data: PageData } = $props();

	let chosen_schedule = $state('0');
	let chosen_playlist = $state('0');
	let type: 'schedule' | 'playlist' = $state('playlist');
</script>

<form
	method="POST"
	action="?/create"
	class="w-full max-w-7xl"
	use:enhance={() => {
		return form_action;
	}}
>
	<div class="grid gap-2 mb-4">
		<Label>Uuid</Label>
		<Input name="uuid" class="input" type="text" placeholder="Uuid for display (optional)" />
	</div>

	<div class="grid gap-2 mb-4">
		<Label>Name</Label>
		<Input required name="name" class="input" type="text" placeholder="Name must be unique" />
	</div>

	<h3 class="mb-2">Display Material</h3>

	<div class="flex md:items-center w-full gap-4 sm:flex-row flex-col">
		<Select.Root name="display_type" type="single" bind:value={type}>
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
			{#if type === 'schedule'}
				<TypePicker label={false} types={data.schedule} bind:chosen_type={chosen_schedule} />
			{:else if type === 'playlist'}
				<TypePicker label={false} types={data.playlist} bind:chosen_type={chosen_playlist} />
			{/if}
		</div>
	</div>
	<input
		type="hidden"
		name="display_uuid"
		value={type === 'schedule' ? chosen_schedule : chosen_playlist}
	/>

	<div class="mt-8">
		<Button type="submit">Add</Button>
	</div>
</form>
