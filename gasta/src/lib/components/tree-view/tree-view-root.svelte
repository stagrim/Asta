<script lang="ts">
	import { setTreeContext } from './ctx.svelte';
	import { cn } from '$lib/utils';
	import type { HTMLAttributes } from 'svelte/elements';

	type Props = HTMLAttributes<HTMLDivElement> & {
		/** Currently selected (highlighted) directory by id */
		selectedId?: string;
	};

	let { selectedId, class: className, children, ...props }: Props = $props();

	const treeState = setTreeContext(selectedId);

	$effect(() => {
		if (selectedId !== undefined) treeState.selectedId = selectedId;
	});

	$effect(() => {
		selectedId = treeState.selectedId;
	});
</script>

<div class={cn('group flex flex-col gap-1', className)} {...props}>
	{@render children?.()}
</div>
