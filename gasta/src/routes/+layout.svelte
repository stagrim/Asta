<script lang="ts">
	import '../app.css';
	import * as Sidebar from '$lib/components/ui/sidebar/index.js';
	import * as Avatar from '$lib/components/ui/avatar/index.js';
	import type { LayoutData } from './$types';
	import Navigation from '$lib/Navigation.svelte';
	import Separator from '$lib/components/ui/separator/separator.svelte';
	import { ModeWatcher } from 'mode-watcher';
	import Button from '$lib/components/ui/button/button.svelte';
	import { MoonIcon } from '@lucide/svelte';
	import { Toaster } from '$lib/components/ui/sonner';
	import { toast } from 'svelte-sonner';
	// import { toastStore } from '$lib/stores';

	let { children, data } = $props();
</script>

<Sidebar.Provider>
	{#if !data.empty}
		<Navigation {data} />
	{/if}
	<Sidebar.Inset>
		<header
			class="bg-background sticky top-0 flex h-16 shrink-0 items-center justify-between gap-2 border-b px-4"
		>
			<div>
				{#if !data.empty}
					<Sidebar.Trigger class="-ml-1" />
				{/if}
			</div>

			<a href="/">
				<Avatar.Root class="w-12 h-12">
					<Avatar.Image src="/asta_icon.jpg" />
					<Avatar.Fallback>AS</Avatar.Fallback>
				</Avatar.Root>
			</a>

			<div>
				<Button variant="outline" size="icon" onclick={() => toast('Not yet implemented')}>
					<MoonIcon class="h-[1.2rem] w-[1.2rem] transition-all!" />
					<span class="sr-only">May someday toggle theme</span>
				</Button>
			</div>
		</header>

		<main class="p-4 flex flex-col items-center">
			{@render children?.()}
		</main>
	</Sidebar.Inset>
</Sidebar.Provider>

<Toaster />
<!--
<Drawer>
	<div class="p-2">
		{#if !data.empty}
			<Navigation {data} />
		{/if}
	</div>
</Drawer> -->

<!-- <AppShell
	slotSidebarLeft={!data.empty ? 'max-w-xs w-0 md:w-2/6 md:p-4 h-screen overflow-y-scroll' : ''}
>
	<svelte:fragment slot="pageHeader">
		<AppBar gridColumns="grid-cols-3" slotDefault="place-self-center" slotTrail="place-content-end">
			<svelte:fragment slot="lead">
				{#if !data.empty}
					<button class="md:hidden btn btn-sm mr-4" on:click={() => drawerStore.open({})}>
						<span>
							<svg viewBox="0 0 100 80" class="fill-token w-4 h-4">
								<rect width="100" height="20" />
								<rect y="30" width="100" height="20" />
								<rect y="60" width="100" height="20" />
							</svg>
						</span>
					</button>
				{/if}
			</svelte:fragment>

			<a href={data.user ? '/' : ''} style:cursor="pointer">
				<Avatar rounded="rounded-full" src="/asta_icon.jpg" width="w-16" />
			</a>

			<svelte:fragment slot="trail">
				<div
					class="lightswitch-track cursor-pointer transition-all duration-[200ms] w-12 h-6 ring-[1px] ring-surface-500/30 rounded-full bg-surface-900"
					role="switch"
					aria-label="Light Switch"
					aria-checked="false"
					title="Toggle Light Mode"
					tabindex="0"
					on:click={() => {
						$toastStore.trigger({
							message: 'Feature not yet implemented',
							timeout: 2000,
							background: 'variant-filled-warning',
							hideDismiss: true
						});
					}}
				>
					<div
						class="lightswitch-thumb aspect-square scale-[0.8] flex justify-center items-center transition-all duration-[200ms] h-6 rounded-full bg-surface-50"
					>
						<svg
							class="lightswitch-icon w-[70%] aspect-square fill-surface-900"
							xmlns="http://www.w3.org/2000/svg"
							viewBox="0 0 512 512"
						>
							<path
								d="M223.5 32C100 32 0 132.3 0 256S100 480 223.5 480c60.6 0 115.5-24.2 155.8-63.4c5-4.9 6.3-12.5 3.1-18.7s-10.1-9.7-17-8.5c-9.8 1.7-19.8 2.6-30.1 2.6c-96.9 0-175.5-78.8-175.5-176c0-65.8 36-123.1 89.3-153.3c6.1-3.5 9.2-10.5 7.7-17.3s-7.3-11.9-14.3-12.5c-6.3-.5-12.6-.8-19-.8z"
							>
							</path>
						</svg>
					</div>
				</div>

			</svelte:fragment>
		</AppBar>
	</svelte:fragment>

	<svelte:fragment slot="sidebarLeft">
		{#if !data.empty}
			<Navigation {data} />
		{/if}
	</svelte:fragment>

	<slot />
</AppShell> -->
