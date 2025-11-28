<script lang="ts">
	import '../app.css';
	import * as Sidebar from '$lib/components/ui/sidebar/index.js';
	import * as Avatar from '$lib/components/ui/avatar/index.js';
	import Navigation from '$lib/Navigation.svelte';
	import Button from '$lib/components/ui/button/button.svelte';
	import { MoonIcon } from '@lucide/svelte';
	import { Toaster } from '$lib/components/ui/sonner';
	import { toast } from 'svelte-sonner';

	let { children, data } = $props();
</script>

<Sidebar.Provider>
	{#if !data.empty}
		<Navigation {data} />
	{/if}
	<Sidebar.Inset class="overflow-auto">
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
