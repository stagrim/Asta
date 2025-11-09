<script lang="ts">
	import * as Card from '$lib/components/ui/card';
	import { enhance } from '$app/forms';
	import { form_action } from '$lib/form_action';
	import TypePicker from '$lib/TypePicker.svelte';
	import type { PageData } from './$types';
	import Label from '$lib/components/ui/label/label.svelte';
	import Input from '$lib/components/ui/input/input.svelte';
	import * as Select from '$lib/components/ui/select/index.js';

	let { data }: { data: PageData } = $props();

	let chosen_schedule = $state('0');
	let chosen_playlist = $state('0');
	let type: 'schedule' | 'playlist' = $state('playlist');
</script>

<form
	class="card m-4"
	method="POST"
	action="?/create"
	use:enhance={() => {
		return form_action;
	}}
>
	<Card.Root>
		<Card.Content>
			<div class="grid gap-2 mb-5">
				<Label>Uuid</Label>
				<Input name="uuid" class="input" type="text" placeholder="Uuid for display (optional)" />
			</div>

			<div class="grid gap-2 mb-5">
				<Label>Name</Label>
				<Input required name="name" class="input" type="text" placeholder="Name must be unique" />
			</div>

			<h1>Display Material</h1>

			<div class="flex md:items-center w-full gap-4 md:flex-row flex-col">
				<Select.Root name="display_type" type="single" bind:value={type}>
					<Select.Trigger class="w-[180px]"
						>{type.charAt(0).toUpperCase()}{type.substring(1)}</Select.Trigger
					>
					<Select.Content>
						<Select.Label>Types</Select.Label>
						<Select.Item value="schedule">Schedule</Select.Item>
						<Select.Item value="playlist">Playlist</Select.Item>
					</Select.Content>
				</Select.Root>
				<div class="w-full">
					{#if type === 'schedule'}
						<TypePicker label={false} types={data.schedule} bind:chosen_type={chosen_schedule} />
					{:else if type === 'playlist'}
						<TypePicker label={false} types={data.playlist} bind:chosen_type={chosen_playlist} />
					{/if}
				</div>
			</div>
		</Card.Content>
	</Card.Root>
	<!-- <section class="p-4">
		<label class="label mb-5">
			<span>Uuid</span>
			<input name="uuid" class="input" type="text" placeholder="Uuid for display (optional)" />
		</label>

		<label class="label mb-5">
			<span>Name</span>
			<input required name="name" class="input" type="text" placeholder="Name must be unique" />
		</label>

		<div class="flex items-center justify-between w-full my-5">
			<h3 class="h3">Display Material</h3>
		</div>

		<div class="flex md:items-center w-full gap-4 md:flex-row flex-col">
			<select class="select w-32" name="display_type" bind:value={type}>
				<option value="schedule">Schedule</option>
				<option value="playlist">Playlist</option>
			</select>
			<div class="w-full">
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

		<div class="flex w-full justify-center mt-5">
			<button type="submit" class="btn variant-filled-primary">Add</button>
		</div>
	</section> -->
</form>
