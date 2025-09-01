<script lang="ts">
	import { Icon } from 'svelte-awesome';
	import plus from 'svelte-awesome/icons/plus';
	import arrowDown from 'svelte-awesome/icons/arrowDown';
	import arrowUp from 'svelte-awesome/icons/arrowUp';
	import trash from 'svelte-awesome/icons/trash';

	import { generate } from 'random-words';
	import { page } from '$app/stores';
	import type { PageData } from './$types';
	import { flip } from 'svelte/animate';
	import UpdateForm from '$lib/UpdateForm.svelte';
	import type { Playlist } from '$lib/api_bindings/read/Playlist';

	export let data: PageData;

	$: uuid = $page.params.uuid;

	let playlist: Playlist;

	let add_value: 'WEBSITE' | 'IMAGE' | 'TEXT';
	const add_item = () =>
		(playlist.items = [
			...playlist.items,
			{
				type: add_value,
				name: generate({ exactly: 1, wordsPerString: 2, separator: '-' })[0],
				settings: { duration: 60 }
			}
		]);

	const swap_item = (a: number, b: number) => {
		const tmp = playlist.items[a];
		playlist.items[a] = playlist.items[b];
		playlist.items[b] = tmp;
	};
</script>

<UpdateForm
	bind:type={data.playlist}
	dependant_state={{ schedules: data.schedule, displays: data.display }}
	bind:uuid
	bind:item={playlist}
>
	{#if playlist}
		<label class="label mb-5">
			<span>Name</span>
			<input
				required
				name="name"
				class="input"
				type="text"
				placeholder="Name must be unique"
				bind:value={playlist.name}
			/>
		</label>

		<div class="flex items-center justify-between w-full my-5">
			<h3 class="h3">Items</h3>

			<div class="flex justify-end items-center w-1/2">
				<!-- <label class="label mb-2 flex justify-center items-center"> -->
				<!-- <span class="mr-2">Add</span> -->
				<select class="select w-3/4" bind:value={add_value}>
					<option value="WEBSITE">Website</option>
					<option value="TEXT">Text</option>
					<option value="IMAGE">Image</option>
				</select>
				<button
					type="button"
					class="btn-icon btn-icon-sm variant-filled-primary ml-2"
					on:click={add_item}
				>
					<Icon data={plus} scale={0.75} />
				</button>
			</div>
			<!-- </label> -->
		</div>

		{#if playlist.items}
			{#each playlist.items as item, i (item.name)}
				<div class="card mb-4 transition-all" animate:flip={{ duration: 300 }}>
					<section class="p-4 lg:flex lg:flex-row-reverse">
						<div class="flex justify-center gap-4 lg:flex-col lg:ml-4">
							<button
								type="button"
								class="btn-icon btn-icon-sm variant-filled-primary"
								class:invisible={i <= 0}
								on:click={() => swap_item(i, i - 1)}
							>
								<Icon data={arrowUp} scale={0.75} />
							</button>
							<button
								type="button"
								class="btn-icon btn-icon-sm variant-filled-error"
								on:click={() => {
									playlist.items.splice(i, 1);
									playlist.items = playlist.items;
								}}
							>
								<Icon data={trash} scale={0.75} />
							</button>
							<button
								type="button"
								class="btn-icon btn-icon-sm variant-filled-primary"
								class:invisible={i >= playlist.items.length - 1}
								on:click={() => swap_item(i, i + 1)}
							>
								<Icon data={arrowDown} scale={0.75} />
							</button>
						</div>
						<div class="w-full">
							<div class="flex items-center gap-3">
								<label class="label mb-5">
									<span>Name</span>
									<input
										required
										class="input"
										type="text"
										placeholder="Name must be unique"
										bind:value={item.name}
									/>
								</label>

								<label class="label mb-5">
									<span>Duration (s)</span>
									<input
										required
										class="input"
										type="number"
										placeholder="Duration in seconds"
										bind:value={item.settings.duration}
									/>
								</label>
							</div>

							{#if item.type == 'WEBSITE'}
								<label class="label mb-5">
									<span>URL</span>
									<input
										required
										class="input"
										type="text"
										placeholder="https://example.com"
										bind:value={item.settings.url}
									/>
								</label>
							{:else if item.type == 'TEXT'}
								<label class="label mb-5">
									<span>Text</span>
									<textarea
										required
										class="input"
										placeholder="Some text..."
										bind:value={item.settings.text}
									></textarea>
								</label>
							{:else if item.type == 'IMAGE'}
								<label class="label mb-5">
									<span>Image source</span>
									<input
										required
										class="input"
										type="text"
										placeholder="https://example.com/src.png"
										bind:value={item.settings.src}
									/>
								</label>
							{/if}
						</div>
					</section>
				</div>
			{/each}
		{/if}
	{/if}
</UpdateForm>
