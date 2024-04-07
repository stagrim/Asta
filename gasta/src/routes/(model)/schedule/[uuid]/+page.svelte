<script lang="ts">
	import { Icon } from 'svelte-awesome';
	import plus from 'svelte-awesome/icons/plus';
	import arrowDown from 'svelte-awesome/icons/arrowDown';
	import arrowUp from 'svelte-awesome/icons/arrowUp';
	import trash from 'svelte-awesome/icons/trash';

	import { page } from '$app/stores';
	import type { PageData } from './$types';
	import type { PlaylistItem } from '$lib/api_bindings/update/PlaylistItem';
	import type { Playlist } from '$lib/api_bindings/read/Playlist';
	import TypePicker from '$lib/TypePicker.svelte';
	import type { Schedule } from '$lib/api_bindings/read/Schedule';
	import { flip } from 'svelte/animate';
	import UpdateForm from '$lib/UpdateForm.svelte';

	export let data: PageData;

	$: uuid = $page.params.uuid;

	let item: Schedule;

	const add_item = () => (item.scheduled = [...(item.scheduled ?? []), {}]);

	const swap_item = (a: number, b: number) => {
		const tmp = item.scheduled![a];
		item.scheduled![a] = item.scheduled![b];
		item.scheduled![b] = tmp;
	};
</script>

<UpdateForm bind:type={data.schedule} bind:dependant_state={data.display} bind:uuid bind:item>
	{#if item}
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

		<TypePicker types={data.playlist} name="playlist" bind:chosen_type={item.playlist} />

		<div class="flex items-center justify-between w-full my-5">
			<h3 class="h3">Scheduled</h3>

			<button
				type="button"
				class="btn-icon btn-icon-sm variant-soft-primary ml-2"
				on:click={add_item}
			>
				<Icon data={plus} scale="0.75" />
			</button>
		</div>

		{#if item.scheduled}
			{#each item.scheduled as scheduled_playlist, i (scheduled_playlist.playlist)}
				<div class="card mb-4" animate:flip={{ duration: 300 }}>
					<header class="card-header">
						<div class="flex w-full justify-center gap-4">
							{#if i > 0}
								<button
									type="button"
									class="btn-icon btn-icon-sm variant-outline-primary"
									on:click={() => swap_item(i, i - 1)}
								>
									<Icon data={arrowUp} scale="0.75" />
								</button>
							{/if}
							<button
								type="button"
								class="btn-icon btn-icon-sm variant-filled-error"
								on:click={() => {
									item.scheduled.splice(i, 1);
									item.scheduled = item.scheduled;
								}}
							>
								<Icon data={trash} />
							</button>
							{#if i < item.scheduled.length - 1}
								<button
									type="button"
									class="btn-icon btn-icon-sm variant-outline-primary"
									on:click={() => swap_item(i, i + 1)}
								>
									<Icon data={arrowDown} scale="0.75" />
								</button>
							{/if}
						</div>
					</header>

					<section class="p-4">
						<TypePicker types={data.playlist} bind:chosen_type={scheduled_playlist.playlist} />

						<label class="label mb-5">
							<span>Start</span>
							<input
								required
								class="input"
								type="text"
								placeholder="ss mm HH dd mm weekday YYYY"
								bind:value={scheduled_playlist.start}
							/>
						</label>

						<label class="label mb-5">
							<span>End</span>
							<input
								required
								class="input"
								type="text"
								placeholder=""
								bind:value={scheduled_playlist.end}
							/>
						</label>
					</section>
				</div>
			{/each}
		{/if}
	{/if}
</UpdateForm>
