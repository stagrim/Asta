<script lang="ts">
	import { enhance } from '$app/forms';
	import { form_action } from '$lib/form_action';
	import TypePicker from '../../lib/TypePicker.svelte';
	import type { PageData } from './$types';

	export let data: PageData;

	let chosen_schedule = '0';
</script>

<form
	class="card m-4"
	method="POST"
	action="?/create"
	use:enhance={() => {
		return form_action;
	}}
>
	<section class="p-4">
		<label class="label mb-5">
			<span>Uuid</span>
			<input name="uuid" class="input" type="text" placeholder="Uuid for display (optional)" />
		</label>

		<label class="label mb-5">
			<span>Name</span>
			<input required name="name" class="input" type="text" placeholder="Name must be unique" />
		</label>

		<TypePicker name="schedule" types={data.schedule} bind:chosen_type={chosen_schedule} />

		<div class="flex w-full justify-center mt-5">
			<!-- Disable button when nothing is changed -->
			<button type="submit" class="btn variant-filled-primary">Add</button>
		</div>
	</section>
</form>
