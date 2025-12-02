<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { IsMobile } from '$lib/hooks/is-mobile.svelte.js';
	import { Label } from '$lib/components/ui/label';
	import { Input } from '$lib/components/ui/input';
	import * as Drawer from '$lib/components/ui/drawer';
	import { Textarea } from '$lib/components/ui/textarea';
	import type { PlaylistItem } from '$lib/api_bindings/update/PlaylistItem';

	const isMobile = new IsMobile();

	let {
		item = $bindable(),
		open = $bindable(false)
	}: { item: PlaylistItem | undefined; open: boolean } = $props();
</script>

<Drawer.Root bind:open direction={isMobile.current ? 'bottom' : 'right'}>
	<Drawer.Content>
		<Drawer.Header class="gap-1">
			<Drawer.Title>Edit Playlist Item</Drawer.Title>
		</Drawer.Header>
		<div class="flex flex-col gap-4 overflow-y-auto px-4 text-sm">
			{#if item}
				<Label>Duration</Label>
				<Input
					required
					class="no-spinner"
					type="number"
					placeholder="Duration in seconds"
					bind:value={item.settings.duration}
				/>

				{#if item.type == 'WEBSITE'}
					<Label>Url</Label>
					<Input
						required
						class="input"
						type="text"
						placeholder="https://example.com"
						bind:value={item.settings.url}
					/>
				{:else if item.type == 'TEXT'}
					<div class="grid w-full gap-1.5">
						<Label for="message">Text</Label>
						<Textarea required placeholder="Some text..." bind:value={item.settings.text} />
					</div>
				{:else if item.type == 'IMAGE'}
					<Label>Image Source</Label>
					<Input
						required
						class="input"
						type="text"
						placeholder="https://example.com/src.png"
						bind:value={item.settings.src}
					/>
				{/if}
			{:else}
				Nothings here
			{/if}
		</div>
		<Drawer.Footer>
			<Drawer.Close>
				{#snippet child({ props })}
					<Button variant="outline" {...props}>Done</Button>
				{/snippet}
			</Drawer.Close>
		</Drawer.Footer>
	</Drawer.Content>
</Drawer.Root>
