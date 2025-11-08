<script lang="ts">
	import * as Sidebar from '$lib/components/ui/sidebar';
	import * as Collapsible from '$lib/components/ui/collapsible';
	import { useSidebar } from '$lib/components/ui/sidebar/index.js';

	import { page } from '$app/state';
	import type { LayoutData } from '../routes/$types';
	import { fade, slide } from 'svelte/transition';
	import sanitizeHtml from 'sanitize-html';
	import type { Display } from './api_bindings/read/Display';
	import type { Schedule } from './api_bindings/read/Schedule';
	import type { Playlist } from './api_bindings/read/Playlist';
	import {
		CalendarClock,
		Camera,
		ChevronRight,
		Funnel,
		ListVideo,
		Monitor,
		Plus,
		SearchIcon
	} from '@lucide/svelte';
	import Label from './components/ui/label/label.svelte';

	let { data }: { data: LayoutData } = $props();

	// const drawerStore = getDrawerStore();
	const sidebar = useSidebar();
	const drawerClose = () => sidebar.openMobile && sidebar.toggle();

	let drawer_open: [boolean, boolean, boolean] = $state([false, false, false]);

	// Only to be able to close drawer menu of the kind you are on. The block below reruns since drawer_open changes
	// when the drawer state is available, making one unable to close the display drawer if the user starts with display
	const set_drawer = (index: number, value = true) => (drawer_open[index] = value);

	$effect(() => {
		if (page.url.pathname.startsWith('/display')) {
			set_drawer(0);
		} else if (page.url.pathname.startsWith('/schedule')) {
			set_drawer(1);
		} else if (page.url.pathname.startsWith('/playlist')) {
			set_drawer(2);
		}
	});
	let filter_value: string = $state('');

	$effect(() => {
		if (filter_value && filter_value !== '') {
			// Open all drawers
			[...Array(3)].forEach((_, i) => set_drawer(i));
		}
	});
	let kinds = $derived(
		[data.display, data.schedule, data.playlist].map((state) => ({
			values: [...state.content.values()].sort((a, b) => a.name.localeCompare(b.name)),
			type: state.type.toLocaleLowerCase()
		}))
	);

	const sanitize_html = (dirty: string) =>
		sanitizeHtml(dirty, {
			allowedTags: [],
			disallowedTagsMode: 'escape'
		});

	function filter_titles(
		kind: ((Display | Schedule | Playlist) & { title_name: string })[],
		filter: string
	) {
		if (filter) {
			const regex = new RegExp(`(${filter})`, 'gi');
			return kind
				.filter(({ title_name }) => regex.test(title_name))
				.map((k) => {
					const copy = Object.assign({}, k);
					regex.lastIndex = 0;

					copy.title_name = k.title_name.replace(
						regex,
						(match) => `<span class="highlight">${match}</span>`
					);

					return copy;
				});
		}
		return kind;
	}
</script>

<Sidebar.Root>
	<Sidebar.Header>
		<Sidebar.Group class="py-0">
			<Sidebar.GroupContent class="relative">
				<Label for="search" class="sr-only">Search</Label>
				<Sidebar.Input id="search" placeholder="Filter" class="pl-8" bind:value={filter_value} />
				<Funnel
					class="pointer-events-none absolute left-2 top-1/2 size-4 -translate-y-1/2 select-none opacity-50"
				/>
			</Sidebar.GroupContent>
		</Sidebar.Group>
	</Sidebar.Header>
	<Sidebar.Content>
		{#each kinds as kind, i (kind.type)}
			{@const capitalized = `${kind.type.charAt(0).toUpperCase()}${kind.type.substring(1)}`}
			{@const sanitized_values = kind.values.map((k) =>
				Object.assign(k, { title_name: sanitize_html(k.name) })
			)}
			<Collapsible.Root title={capitalized} class="group/collapsible" bind:open={drawer_open[i]}>
				<Sidebar.Group>
					<Sidebar.GroupLabel
						class="group/label text-sidebar-foreground hover:bg-sidebar-accent hover:text-sidebar-accent-foreground text-sm"
					>
						{#snippet child({ props })}
							<Collapsible.Trigger {...props}>
								{#if kind.type == 'display'}
									<Monitor />
								{:else if kind.type == 'schedule'}
									<CalendarClock />
								{:else}
									<ListVideo />
								{/if}
								<span class="ml-1">{capitalized}</span>

								<ChevronRight
									class="ml-auto transition-transform group-data-[state=open]/collapsible:rotate-90"
								/>
							</Collapsible.Trigger>
						{/snippet}
					</Sidebar.GroupLabel>
					<Collapsible.Content>
						<Sidebar.GroupContent>
							<Sidebar.MenuSub>
								{#each filter_titles(sanitized_values, filter_value) as { uuid, title_name } (uuid)}
									{@const href = `/${kind.type}/${uuid}`}
									<Sidebar.MenuSubItem>
										<Sidebar.MenuSubButton>
											{#snippet child({ props })}
												<a {href} onclick={drawerClose} {...props}>
													<span>{@html title_name}</span>
												</a>
											{/snippet}
										</Sidebar.MenuSubButton>
									</Sidebar.MenuSubItem>
								{/each}
								<Sidebar.MenuItem>
									<Sidebar.MenuButton>
										{#snippet child({ props })}
											<a href="/{kind.type}" onclick={drawerClose} {...props}>
												<Plus />
												<span>Add {capitalized}</span>
											</a>
										{/snippet}
									</Sidebar.MenuButton>
								</Sidebar.MenuItem>
							</Sidebar.MenuSub>
						</Sidebar.GroupContent>
					</Collapsible.Content>
				</Sidebar.Group>
			</Collapsible.Root>
			<!-- <Accordion.Root type="single">
				<Accordion.Item value="item-1">
					<Accordion.Trigger>Is it accessible?</Accordion.Trigger>
					<Accordion.Content>Yes. It adheres to the WAI-ARIA design pattern.</Accordion.Content>
				</Accordion.Item>
			</Accordion.Root> -->
		{/each}
	</Sidebar.Content>
</Sidebar.Root>

<!-- <nav class="list-nav"> -->
<!-- <div class="input-group input-group-divider grid-cols-[1fr_auto] mb-1">
		<input type="hidden" autofocus={true} />

		<input type="text" class="input" placeholder="filter" bind:value={filter_value} />
		{#if filter_value === undefined || filter_value === ''}
			<div class="input-group-shim" in:fade><Icon data={filter}></Icon></div>
		{:else}
			<button class="input-group-divider" in:fade onclick={() => (filter_value = '')}
				><Icon data={faDeleteLeft}></Icon></button
			>
		{/if}
	</div> -->

<!-- <Accordion>
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
								class:variant-filled-primary={href === page.url.pathname}
							>
								<a {href} onclick={drawerClose}>
									<span class="flex-auto p-1 whitespace-pre-wrap leading-5">{@html title_name}</span
									>
								</a>
							</li>
						{/each}

						<li>
							<a
								href={`/${kind.type}/`}
								onclick={drawerClose}
								class={`/${kind.type}` === page.url.pathname ? 'variant-glass-primary' : ''}
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
	</Accordion> -->
<!-- </nav> -->

<style>
	:global(:root [data-theme='d-theme']) {
		--highlight-color: #404040;
		--highlight-color-active: rgba(var(--color-surface-500) / 0.5);
	}
	:global(span.highlight) {
		background-color: var(--highlight-color);
		box-shadow: 0 0 0 1px var(--highlight-color);
		animation: pulse 0.2s normal;
		border-radius: 0.15rem;
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
