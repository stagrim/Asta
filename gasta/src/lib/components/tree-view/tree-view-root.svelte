<script lang="ts">
	import { setTreeContext } from './ctx.svelte';
	import { cn } from '$lib/utils';
	import type { HTMLAttributes } from 'svelte/elements';
	import type { TreeFile } from '$lib/server/sasta_client';

	type Props = HTMLAttributes<HTMLDivElement> & {
		/** Currently selected (highlighted) directory by id */
		selectedId?: string;
		complementryData?: TreeFile;
	};

	let {
		selectedId = $bindable(),
		complementryData = $bindable(),
		class: className,
		children,
		...props
	}: Props = $props();

	const treeState = setTreeContext(selectedId);

	$effect(() => {
		if (selectedId !== undefined) treeState.selectedId = selectedId;
	});
	$effect(() => {
		if (complementryData !== undefined) treeState.complementryData = complementryData;
	});

	$effect(() => {
		selectedId = treeState.selectedId;
		complementryData = treeState.complementryData;
	});
</script>

<div class={cn('group flex flex-col gap-1', className)} {...props}>
	{@render children?.()}
</div>
