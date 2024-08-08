<script lang="ts">
	import { Icon } from 'svelte-awesome';
	import plus from 'svelte-awesome/icons/plus';
	import arrowDown from 'svelte-awesome/icons/arrowDown';
	import arrowUp from 'svelte-awesome/icons/arrowUp';
	import trash from 'svelte-awesome/icons/trash';
	import listUl from 'svelte-awesome/icons/listUl';

	import { page } from '$app/stores';
	import type { PageData } from './$types';
	import type { PlaylistItem } from '$lib/api_bindings/update/PlaylistItem';
	import type { Playlist } from '$lib/api_bindings/read/Playlist';
	import TypePicker from '$lib/TypePicker.svelte';
	import type { Schedule } from '$lib/api_bindings/read/Schedule';
	import { flip } from 'svelte/animate';
	import UpdateForm from '$lib/UpdateForm.svelte';
	import CronField from './CronField.svelte';
	import { onMount } from 'svelte';

	export let data: PageData;

	$: uuid = $page.params.uuid;

	let item: Schedule;

	const add_item = () => (item.scheduled = [...(item.scheduled ?? []), {}]);

	const swap_item = (a: number, b: number) => {
		const tmp = item.scheduled![a];
		item.scheduled![a] = item.scheduled![b];
		item.scheduled![b] = tmp;
	};

	/** Takes function which may throw an error and catches it.
	 * Executes the function and returns true if no error is thrown, and false if an error is caught.
	 * */
	const throwless = (fn: Function) => {
		try {
			fn();
			return true;
		} catch (e) {
			console.log(e);
			return false;
		}
	};

	let valid_crons: boolean[] = [];

	const set_valid = (index: number, value: boolean) => {
		valid_crons[index] = value;
	};

	$: update_enabled = valid_crons.every((a) => a);

	$: schedule_info = data.schedule_info;
	let timeout_handle;

	const clear_timeout = () => (timeout_handle ? timeout_handle.cancel() : null);
	// Clear current timeout if server load function has changed schedule_info on navigation
	$: data.schedule_info, clear_timeout();

	/**
	 * Cancelable delay object
	 * @param delay delay in milliseconds
	 */
	const later = (delay) => {
		let timer = 0;
		let reject = null;
		const promise = new Promise((resolve, _reject) => {
			reject = _reject;
			timer = setTimeout(resolve, delay);
		});
		return {
			/** Returns the promise to be awaited */
			get promise() {
				return promise;
			},
			/** Clears the delay and throws an error */
			cancel() {
				if (timer) {
					clearTimeout(timer);
					timer = 0;
					reject();
					reject = null;
				}
			}
		};
	};

	const pathname = () => `/schedule/${uuid}`;

	const updateLoop = async () => {
		while (true) {
			const sleep = schedule_info.next?.in_ms ?? 9999999999;
			timeout_handle = later(sleep);
			try {
				await timeout_handle.promise;
			} catch {}

			try {
				schedule_info = await (await fetch(pathname())).json();
			} catch (e) {
				console.error(e);
			}
		}
	};

	onMount(() => {
		updateLoop();
	});
</script>

<UpdateForm
	bind:type={data.schedule}
	bind:dependant_state={data.display}
	bind:uuid
	bind:item
	bind:update_enabled
>
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

		<div
			class={`rounded-md transition-all ${
				schedule_info.current === item.playlist ? 'border-4 border-primary-500' : 'p-[4px]'
			}`}
		>
			<div class="p-2">
				<TypePicker types={data.playlist} name="playlist" bind:chosen_type={item.playlist} />
			</div>
		</div>

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
			{#if item.scheduled.length !== 0}
				<div class="card m-4 p-4">
					{#if schedule_info.next}
						{@const next_playlist = data.playlist.content.get(schedule_info.next.playlist)}
						{@const change_date = new Date(Date.now() + schedule_info.next.in_ms)}
						{#if next_playlist}
							Will change to
							<a class="mx-1 top-2 align-text-top" href={`/playlist/${next_playlist.uuid}`}>
								<span class="chip variant-ghost-primary gap-1"
									><Icon data={listUl}></Icon> &nbsp;{next_playlist.name}</span
								>
							</a>
							at {change_date.toLocaleString()}
						{:else}
							Playlist {schedule_info.next.playlist} does not exist :(
						{/if}
					{:else}
						<i class="text-surface-300">No playlist change is scheduled</i>
					{/if}
				</div>
			{/if}

			{#each item.scheduled as scheduled_playlist, i (scheduled_playlist)}
				<div
					class={`card mb-4 transition-all ${
						schedule_info.current === scheduled_playlist.playlist
							? 'border-4 border-primary-500'
							: 'p-[4px]'
					}`}
					animate:flip={{ duration: 300 }}
				>
					<section class="p-4 lg:flex lg:flex-row-reverse">
						<div class="flex justify-center gap-4 lg:flex-col lg:ml-4">
							<button
								type="button"
								class="btn-icon btn-icon-sm variant-filled-primary"
								class:invisible={i <= 0}
								on:click={() => swap_item(i, i - 1)}
							>
								<Icon data={arrowUp} scale="0.75" />
							</button>
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
							<button
								type="button"
								class="btn-icon btn-icon-sm variant-filled-primary"
								class:invisible={i >= item.scheduled.length - 1}
								on:click={() => swap_item(i, i + 1)}
							>
								<Icon data={arrowDown} scale="0.75" />
							</button>
						</div>

						<div class="w-full">
							<TypePicker types={data.playlist} bind:chosen_type={scheduled_playlist.playlist} />
							<div class="lg:flex gap-6 mt-2">
								<CronField
									title={'Start'}
									index={2 * i}
									{set_valid}
									bind:value={scheduled_playlist.start}
								></CronField>

								<CronField
									title={'End'}
									index={2 * i + 1}
									{set_valid}
									bind:value={scheduled_playlist.end}
								></CronField>
							</div>
						</div>
					</section>
				</div>
			{/each}
		{/if}
	{/if}
</UpdateForm>
