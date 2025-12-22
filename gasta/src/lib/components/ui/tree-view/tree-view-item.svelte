<script lang="ts">
	import { getTreeContext } from './ctx.svelte';
	import { cn } from '$lib/utils';
	import { File } from '@lucide/svelte';
	import type { HTMLAttributes } from 'svelte/elements';

	type Props = HTMLAttributes<HTMLButtonElement> & {
		value: string; // Unique ID for selection
		label?: string; // Optional: Pass label string or use children slot
	};

	let { value, label, class: className, children, ...props }: Props = $props();

	const ctx = getTreeContext();
	let isSelected = $derived(ctx.selectedId === value);
</script>

<button
	type="button"
	onclick={() => ctx.select(value)}
	class={cn(
		'relative flex h-8 w-full select-none items-center gap-2 rounded-md px-2 text-sm font-medium transition-colors hover:bg-accent hover:text-accent-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:opacity-50',
		isSelected && 'bg-accent text-accent-foreground',
		!isSelected && 'text-foreground/80',
		className
	)}
	{...props}
>
	<File class="size-4 shrink-0 text-muted-foreground" />
	{#if children}
		{@render children()}
	{:else}
		<span class="truncate">{label ?? value}</span>
	{/if}
</button>
