<script lang="ts">
	import TypePicker from '#lib/TypePicker.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import { Label } from '#lib/components/ui/label/index.js';
	import type { PageData } from './$types';
	import { createSchedule } from './schedules.remote';
	import { toast } from 'svelte-sonner';

	let { data }: { data: PageData } = $props();

	const { name, playlist } = createSchedule.fields;
</script>

<form
	class="w-full max-w-7xl"
	{...createSchedule.enhance(async ({ submit }) => {
		try {
			if (await submit()) {
				toast.success('Schedule Added');
			} else {
				toast.error(createSchedule.fields.allIssues()?.[0]?.message || 'Failed to add schedule');
			}
		} catch (error: any) {
			toast.error(error.body?.message || 'Failed to add schedule');
		}
	})}
>
	<div class="grid gap-2 mb-5">
		<Label>Name</Label>
		<Input {...name.as('text')} required class="input" placeholder="Name must be unique" />
	</div>

	<TypePicker types={data.playlist} {...playlist.as('select')} />

	<div class="mt-8">
		<Button type="submit">Add</Button>
	</div>
</form>
