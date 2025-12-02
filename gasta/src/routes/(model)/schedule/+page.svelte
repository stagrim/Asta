<script lang="ts">
	import { enhance } from '$app/forms';
	import TypePicker from '$lib/TypePicker.svelte';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Label } from '$lib/components/ui/label';
	import { form_action } from '$lib/form_action';
	import type { PageData } from './$types';

	export let data: PageData;
</script>

<form class="w-full max-w-7xl" method="POST" action="?/create" use:enhance={() => form_action}>
	<div class="grid gap-2 mb-5">
		<Label>Name</Label>
		<Input required name="name" class="input" type="text" placeholder="Name must be unique" />
	</div>

	<TypePicker types={data.playlist} name="playlist" />

	<div class="mt-8">
		<Button type="submit">Add</Button>
	</div>
</form>
