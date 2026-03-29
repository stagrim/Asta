<script lang="ts">
	import { getTreeContext } from './ctx.svelte';
	import { cn } from '$lib/utils';
	import type { HTMLAttributes } from 'svelte/elements';
	import FileIcon from '../file-manager/FileIcon.svelte';
	import type { TreeFile } from '$lib/server/sasta_client';

	type Props = HTMLAttributes<HTMLButtonElement> & {
		value: string; // Unique ID for selection
		label?: string; // Optional: Pass label string or use children slot
		complementryData?: TreeFile; // Optional: complementry data to the selected item
	};

	let { value, label, class: className, children, complementryData, ...props }: Props = $props();

	const ctx = getTreeContext();
	let isSelected = $derived(ctx.selectedId === value);
</script>

<button
	type="button"
	onclick={() => ctx.select(value, complementryData)}
	class={cn(
		'relative flex h-8 w-full select-none items-center gap-2 rounded-md px-2 text-sm font-medium transition-colors hover:bg-accent/50 hover:text-accent-foreground focus-visible:outline-none disabled:opacity-50',
		isSelected && 'bg-accent text-accent-foreground',
		!isSelected && 'text-foreground/80',
		className
	)}
	{...props}
>
	<FileIcon extension={value.split('.').at(-1)} size="sm" />
	{#if children}
		{@render children()}
	{:else}
		<span class="truncate">{label ?? value}</span>
	{/if}
</button>
