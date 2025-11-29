<script lang="ts">
	import * as Sidebar from '$lib/components/ui/sidebar';
	import * as Collapsible from '$lib/components/ui/collapsible';
	import { useSidebar } from '$lib/components/ui/sidebar/index.js';

	import { page } from '$app/state';
	import type { LayoutData } from '../routes/$types';
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
	import { capitalize } from './utils';

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
			{@const capitalized = capitalize(kind.type)}
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
												{@const active = href === page.url.pathname}
												<a {href} onclick={drawerClose} {...props}>
													<div
														class="w-1 h-7/12 rounded-2xl bg-foreground transition-transform"
														class:scale-y-100={active}
														class:scale-y-0={!active}
													></div>
													<span>{@html title_name}</span>
												</a>
											{/snippet}
										</Sidebar.MenuSubButton>
									</Sidebar.MenuSubItem>
								{/each}
								<Sidebar.MenuItem>
									<Sidebar.MenuButton>
										{#snippet child({ props })}
											{@const href = `/${kind.type}`}
											{@const active = href === page.url.pathname}
											<a {href} onclick={drawerClose} {...props}>
												<div
													class="w-1 h-full rounded-2xl bg-foreground transition-transform"
													class:scale-y-100={active}
													class:scale-y-0={!active}
												></div>
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
		{/each}
	</Sidebar.Content>
</Sidebar.Root>

<style>
	:global(:root) {
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
