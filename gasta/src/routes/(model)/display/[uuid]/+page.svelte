<script lang="ts">
	import { page } from '$app/stores';
	import TypePicker from '$lib/TypePicker.svelte';
	import UpdateForm from '$lib/UpdateForm.svelte';
	import { toastStore } from '$lib/stores';
	import type { PageData } from './$types';
	import type { Display } from '$lib/api_bindings/read/Display';

	let { data }: { data: PageData } = $props();

	let uuid = $derived($page.params.uuid);

	let item: Display | undefined = $state(undefined);
	let other_uuid = $state('');
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_missing_attribute -->
<UpdateForm bind:type={data.display} {uuid} bind:item>
	{#if item}
		<span class="mb-5">
			<span>Uuid</span>
			<abbr title="Click to copy UUID">
				<div
					class="input-group input-group-divider grid-cols-[1fr_auto] cursor-pointer"
					onclick={async (_) => {
						try {
							await navigator.clipboard.writeText(item!.uuid);
							$toastStore.trigger({
								message: 'Display Uuid copied to clipboard',
								background: 'variant-filled-primary',
								timeout: 2000,
								hideDismiss: true
							});
						} catch (err) {
							$toastStore.trigger({
								message: 'Could not copy Uuid, ' + err,
								background: 'variant-filled-error',
								autohide: false
							});
						}
					}}
				>
					<input class="input text-surface-400" type="text" readonly value={item.uuid} />
					<a><Icon data={copy} scale={0.75} /></a>
				</div>
			</abbr>
		</span>

		<label class="label mb-5">
			<span>Name</span>
			<input
				required
				name="name"
				class="input"
				type="text"
				placeholder="Name must be unique"
				bind:value={item.name}
			/>
		</label>

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
