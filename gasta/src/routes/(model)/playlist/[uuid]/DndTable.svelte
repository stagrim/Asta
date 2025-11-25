<script lang="ts" module>
	export type ColumnDef<T> = {
		id: string;
		label: string;
		class?: string;
	} & (
		| {
				render: (item: T) => RenderSnippetConfig<{ item: T }>;
				accessorKey?: never;
		  }
		| {
				accessorKey: string;
				render?: never;
		  }
	);

	/**
	 * A helper function to help create cells from Svelte Snippets through ColumnDef's `cell` and `header` properties.
	 *
	 * The snippet must only take one parameter.
	 *
	 * @param snippet
	 * @param params
	 * @returns - A `RenderSnippetConfig` object that helps svelte-table know how to render the header/cell snippet.
	 * @example
	 * ```ts
	 * // +page.svelte
	 * const defaultColumns = [
	 *   columnHelper.accessor('name', {
	 *     cell: cell => renderSnippet(nameSnippet, { name: cell.row.name }),
	 *   }),
	 *   columnHelper.accessor('state', {
	 *     cell: cell => renderSnippet(stateSnippet, { state: cell.row.state }),
	 *   }),
	 * ]
	 * ```
	 */
	export function renderSnippet<TProps>(snippet: Snippet<[TProps]>, params: TProps = {} as TProps) {
		return new RenderSnippetConfig(snippet, params);
	}

	/**
	 * A helper class to make it easy to identify Svelte Snippets in `columnDef.cell` and `columnDef.header` properties.
	 *
	 * > NOTE: This class should only be used internally by the adapter. If you're
	 * reading this and you don't know what this is for, you probably don't need it.
	 *
	 * @example
	 * ```svelte
	 * {@const result = content(context as any)}
	 * {#if result instanceof RenderSnippetConfig}
	 *   {@const { snippet, params } = result}
	 *   {@render snippet(params)}
	 * {/if}
	 * ```
	 */
	export class RenderSnippetConfig<TProps> {
		snippet: Snippet<[TProps]>;
		params: TProps;
		constructor(snippet: Snippet<[TProps]>, params: TProps) {
			this.snippet = snippet;
			this.params = params;
		}
	}
</script>

<script lang="ts" generics="TData">
	import { flip } from 'svelte/animate';
	import { dragHandle, dragHandleZone, type DndEvent } from 'svelte-dnd-action';
	import { GripVerticalIcon } from '@lucide/svelte';
	import { cn } from '$lib/utils';
	import type { PlaylistItem } from '$lib/api_bindings/update/PlaylistItem';
	import DataTableCellEditor from './DataTableCellEditor.svelte';
	import { overrideItemIdKeyNameBeforeInitialisingDndZones } from 'svelte-dnd-action';
	import type { Snippet } from 'svelte';
	overrideItemIdKeyNameBeforeInitialisingDndZones('name');

	type Item = TData & { name: string };

	let {
		data = $bindable(),
		columns,
		flipDurationMs = 300,
		editorOpen = $bindable(false),
		editorItem = $bindable(undefined)
	}: {
		data: Item[];
		columns: ColumnDef<Item>[];
		flipDurationMs?: number;
		editorOpen: boolean;
		editorItem: PlaylistItem | undefined;
	} = $props();

	function handleDndConsider(e: CustomEvent<DndEvent<Item>>) {
		data = e.detail.items;
	}

	function handleDndFinalize(e: CustomEvent<DndEvent<Item>>) {
		data = e.detail.items;
	}
</script>

<DataTableCellEditor bind:open={editorOpen} bind:item={editorItem} />

<div class="w-full overflow-auto border rounded-lg shadow-sm bg-card text-card-foreground">
	<table class="w-full caption-bottom text-sm">
		<thead class="[&_tr]:border-b bg-muted">
			<tr class="border-b transition-colors hover:bg-muted/50 data-[state=selected]:bg-muted">
				<th
					class="text-foreground h-10 whitespace-nowrap bg-clip-padding px-2 text-left align-middle font-medium [&:has([role=checkbox])]:pr-0"
				>
					<span class="sr-only">Reorder</span>
				</th>

				{#each columns as col}
					<th
						class={cn(
							'text-foreground h-10 whitespace-nowrap bg-clip-padding px-2 text-left align-middle font-medium [&:has([role=checkbox])]:pr-0',
							col.class
						)}
					>
						{col.label}
					</th>
				{/each}
			</tr>
		</thead>

		<tbody
			class="[&_tr:last-child]:border-0"
			use:dragHandleZone={{ items: data, flipDurationMs, dropTargetStyle: {} }}
			onconsider={handleDndConsider}
			onfinalize={handleDndFinalize}
		>
			{#each data as item (item.name)}
				<tr
					animate:flip={{ duration: flipDurationMs }}
					class="hover:[&,&>svelte-css-wrapper]:[&>th,td]:bg-muted/50 data-[state=selected]:bg-muted border-b transition-colors"
				>
					<!-- Drag Handle Cell -->
					<td
						use:dragHandle
						class="whitespace-nowrap bg-clip-padding p-2 align-middle [&:has([role=checkbox])]:pr-0"
					>
						<div
							class="cursor-grab active:cursor-grabbing text-muted-foreground hover:text-foreground flex items-center justify-center w-8 h-8 rounded hover:bg-muted"
						>
							<GripVerticalIcon class="text-muted-foreground size-3" />
						</div>
					</td>

					{#each columns as col}
						<td
							class={cn(
								'whitespace-nowrap bg-clip-padding p-2 align-middle [&:has([role=checkbox])]:pr-0',
								col.class
							)}
						>
							{#if col.render}
								{@const result = col.render(item) as any}
								{#if result instanceof RenderSnippetConfig}
									{@const { snippet, params } = result}
									{@render snippet(params)}
								{/if}
							{:else}
								{(item as any)[col.accessorKey]}
							{/if}
						</td>
					{/each}
				</tr>
			{/each}
		</tbody>
	</table>
</div>
