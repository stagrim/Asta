<script lang="ts">
	import { page } from '$app/stores';
	import TypePicker from '$lib/TypePicker.svelte';
	import UpdateForm from '$lib/UpdateForm.svelte';
	import { toastStore } from '$lib/stores';
	import type { PageData } from './$types';
	import type { Display } from '$lib/api_bindings/read/Display';
	import { Label } from '$lib/components/ui/label';
	import { Input } from '$lib/components/ui/input';
	import * as InputGroup from '$lib/components/ui/input-group';
	import { CheckIcon, CopyIcon } from '@lucide/svelte';
	import { toast } from 'svelte-sonner';

	let { data }: { data: PageData } = $props();

	let uuid = $derived($page.params.uuid);

	let item: Display | undefined = $state(undefined);
	let other_uuid = $state('');

	let copied = $state(false);
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_missing_attribute -->

<UpdateForm bind:type={data.display} {uuid} bind:item>
	{#if item}
		<div class="grid gap-2 mb-5">
			<Label>Uuid</Label>
			<InputGroup.Root>
				<InputGroup.Input value={item.uuid} class="text-zinc-500" readonly />
				<InputGroup.Addon align="inline-end">
					<InputGroup.Button
						aria-label="Copy"
						title="Copy"
						size="icon-xs"
						onclick={async () => {
							try {
								await navigator.clipboard.writeText(item!.uuid);
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

		<div class="grid gap-2 mb-5">
			<Label>Name</Label>
			<Input
				required
				name="name"
				class="input"
				type="text"
				placeholder="Name must be unique"
				bind:value={item.name}
			/>
		</div>

		<div class="flex items-center w-full gap-4">
			<select
				class="select w-32"
				bind:value={
					() => item?.display_material.type,
					(v) => {
						if (item && v) {
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
				<option value="schedule">Schedule</option>
				<option value="playlist">Playlist</option>
			</select>
			<div class="w-full">
				<TypePicker
					label={false}
					name="display_material"
					types={item.display_material.type === 'schedule' ? data.schedule : data.playlist}
					bind:chosen_type={item.display_material.uuid}
				/>
			</div>
		</div>
	{/if}
</UpdateForm>

<style>
	input[readonly] {
		cursor: unset !important;
	}
</style>
