<script lang="ts">
	import { Icon } from 'svelte-awesome';
	import tv from 'svelte-awesome/icons/tv';
	import calendar from 'svelte-awesome/icons/calendar';
	import filter from 'svelte-awesome/icons/filter';
	import listUl from 'svelte-awesome/icons/listUl';
	import plus from 'svelte-awesome/icons/plus';
	import { faDeleteLeft } from '@fortawesome/free-solid-svg-icons/faDeleteLeft';

	import { page } from '$app/stores';
	import { Accordion, AccordionItem, getDrawerStore } from '@skeletonlabs/skeleton';
	import type { LayoutData } from '../routes/$types';
	import { fade, slide } from 'svelte/transition';
	import sanitizeHtml from 'sanitize-html';
	import type { Display } from './api_bindings/read/Display';
	import type { Schedule } from './api_bindings/read/Schedule';
	import type { Playlist } from './api_bindings/read/Playlist';

	export let data: LayoutData;

	const drawerStore = getDrawerStore();

	const drawerClose = () => drawerStore.close();

	let drawer_open: [boolean, boolean, boolean] = [false, false, false];

	// Only to be able to close drawer menu of the kind you are on. The block below reruns since drawer_open changes
	// when the drawer state is available, making one unable to close the display drawer if the user starts with display
	const set_drawer = (index: number, value = true) => (drawer_open[index] = value);

	$: if ($page.url.pathname.startsWith('/display')) {
		set_drawer(0);
	} else if ($page.url.pathname.startsWith('/schedule')) {
		set_drawer(1);
	} else if ($page.url.pathname.startsWith('/playlist')) {
		set_drawer(2);
	}
	let filter_value: string;

	$: if (filter_value && filter_value !== '') {
		// Open all drawers
		[...Array(3)].forEach((_, i) => set_drawer(i));
	}
	$: kinds = [data.display, data.schedule, data.playlist].map((state) => ({
		values: [...state.content.values()].sort((a, b) => a.name.localeCompare(b.name)),
		type: state.type.toLocaleLowerCase()
	}));

	const sanitize_html = (dirty: string) =>
		sanitizeHtml(dirty, {
			allowedTags: [],
			disallowedTagsMode: 'escape'
		});

	const filter_titles = (
		kind: ((Display | Schedule | Playlist) & { title_name: string })[],
		filter: string
	) => {
		if (filter) {
			return kind
				.filter(({ title_name }) => RegExp(`(${filter})`, 'gi').test(title_name))
				.map((k) =>
					Object.assign(k, {
						title_name: k.title_name.replace(
							RegExp(`(${filter})`, 'gi'),
							'<span class="highlight">$1</span>'
						)
					})
				);
		}
		return kind;
	};
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<!-- svelte-ignore a11y-autofocus -->
<nav class="list-nav">
	<div class="input-group input-group-divider grid-cols-[1fr_auto] mb-1">
		<!-- Stupid solution to avoid having mobiles autofocus and bring up the keyboard on the filter input field -->
		<input type="hidden" autofocus={true} />

		<input type="text" class="input" placeholder="filter" bind:value={filter_value} />
		{#if filter_value === undefined || filter_value === ''}
			<div class="input-group-shim" in:fade><Icon data={filter}></Icon></div>
		{:else}
			<button class="input-group-divider" in:fade on:click={() => (filter_value = '')}
				><Icon data={faDeleteLeft}></Icon></button
			>
		{/if}
	</div>
	<Accordion>
		{#each kinds as kind, i (kind.type)}
			{@const capitalized = `${kind.type.charAt(0).toUpperCase()}${kind.type.substring(1)}`}
			{@const sanitized_values = kind.values.map((k) =>
				Object.assign(k, { title_name: sanitize_html(k.name) })
			)}
			<AccordionItem bind:open={drawer_open[i]}>
				<svelte:fragment slot="lead"></svelte:fragment>
				<svelte:fragment slot="summary">
					<h3 class="h3 flex items-center gap-2">
						<Icon
							data={kind.type == 'display' ? tv : kind.type == 'schedule' ? calendar : listUl}
						/>
						{capitalized}s
					</h3>
				</svelte:fragment>
				<svelte:fragment slot="content">
					<ul>
						{#each filter_titles(sanitized_values, filter_value) as { uuid, title_name } (uuid)}
							{@const href = `/${kind.type}/${uuid}`}
							<li
								transition:slide={{ delay: 50, duration: 150 }}
								class="overflow-hidden rounded-container-token"
								class:variant-filled-primary={href === $page.url.pathname}
							>
								<a {href} on:click={drawerClose}>
									<span class="flex-auto p-1 whitespace-pre-wrap leading-5">{@html title_name}</span
									>
								</a>
							</li>
						{/each}

						<li>
							<a
								href={`/${kind.type}/`}
								on:click={drawerClose}
								class={`/${kind.type}` === $page.url.pathname ? 'variant-glass-primary' : ''}
							>
								<span class="btn-icon btn-icon-sm variant-soft-primary">
									<Icon data={plus} scale={0.75} />
								</span>
								<span class="flex-auto py-1 whitespace-pre-wrap">Add {capitalized}</span>
							</a>
						</li>
					</ul>
				</svelte:fragment>
			</AccordionItem>
		{/each}
	</Accordion>
</nav>

<style lang="postcss">
	:global(:root [data-theme='d-theme']) {
		--highlight-color: rgba(var(--color-primary-500) / 0.5);
		--highlight-color-active: rgba(var(--color-surface-500) / 0.5);
	}
	:global(span.highlight) {
		background-color: var(--highlight-color);
		box-shadow: 0 0 0 1px var(--highlight-color);
		animation: pulse 0.2s normal;
		border-radius: var(--theme-rounded-base);
	}

	:global(.variant-filled-primary span.highlight) {
		background-color: var(--highlight-color-active);
		box-shadow: 0 0 0 1px var(--highlight-color-active);
		animation: pulse-active 0.2s normal;
	}

	@keyframes pulse {
		0% {
			/* -moz-box-shadow: 0 0 0 0 var(--highlight-color); */
			box-shadow: 0 0 0 1px var(--highlight-color);
		}
		50% {
			/* -moz-box-shadow: 0 0 0 10px var(--highlight-color); */
			box-shadow: 0 0 0 2px var(--highlight-color);
		}
		100% {
			/* -moz-box-shadow: 0 0 0 0 var(--highlight-color); */
			box-shadow: 0 0 0 1px var(--highlight-color);
		}
	}

	@keyframes pulse-active {
		0% {
			/* -moz-box-shadow: 0 0 0 0 var(--highlight-color-active); */
			box-shadow: 0 0 0 1px var(--highlight-color-active);
		}
		50% {
			/* -moz-box-shadow: 0 0 0 10px var(--highlight-color-active); */
			box-shadow: 0 0 0 2px var(--highlight-color-active);
		}
		100% {
			/* -moz-box-shadow: 0 0 0 0 var(--highlight-color-active); */
			box-shadow: 0 0 0 1px var(--highlight-color-active);
		}
	}
</style>
