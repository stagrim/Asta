<script lang="ts">
	import { Folder, FolderOpen } from '@lucide/svelte';
	import { cn } from '$lib/utils';
	import { getTreeContext } from './ctx.svelte';
	import * as Collapsible from '../collapsible';
	import { buttonVariants } from '../button';
	import type { CollapsibleTriggerProps } from 'bits-ui';

	type Props = CollapsibleTriggerProps & {
		/** Optional: If folders are selectable */
		id?: string;
		name: string;
		open?: boolean;
	};

	let { id, name, open, class: className, children, ...props }: Props = $props();

	const ctx = getTreeContext();
	let isSelected = $derived(id && ctx.selectedId === id);
	let IconComponent = $derived(open ? FolderOpen : Folder);

	$effect(() => {
		if (id && ctx.selectedId?.startsWith(id)) {
			open = true;
		}
	});
</script>

<Collapsible.Root bind:open class={cn('w-full', className)}>
	<Collapsible.Trigger
		class={cn(
			'flex items-center justify-center p-0.5 rounded-sm w-full',
			buttonVariants({ variant: 'ghost', size: 'sm' }),
			isSelected && 'bg-accent text-accent-foreground',
			!isSelected && 'text-foreground/80'
		)}
		{...props}
	>
		<div class="relative flex w-full select-none items-center gap-2 transition-colors">
			<!-- svelte-ignore a11y_click_events_have_key_events -->
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div class="flex flex-1 gap-2 cursor-pointer">
				<IconComponent class="size-4 shrink-0 text-muted-foreground" />

				<span class="truncate mr-5">{name}</span>
			</div>
		</div>
	</Collapsible.Trigger>

	<Collapsible.Content
		class="overflow-hidden data-[state=closed]:animate-collapse-up data-[state=open]:animate-collapse-down"
	>
		{#if children?.length}
			<div class="relative ml-2 border-l border-muted pl-2 flex flex-col gap-1 py-1">
				{@render children()}
			</div>
		{/if}
	</Collapsible.Content>
</Collapsible.Root>
