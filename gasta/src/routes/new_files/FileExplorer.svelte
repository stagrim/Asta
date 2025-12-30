<script lang="ts">
	import FileIcon from './FileIcon.svelte';
	import { Button } from '$lib/components/ui/button';
	import { ScrollArea } from '$lib/components/ui/scroll-area';
	import { Separator } from '$lib/components/ui/separator';
	import LayoutGridIcon from '@lucide/svelte/icons/layout-grid';
	import ListIcon from '@lucide/svelte/icons/list';
	import PanelRightIcon from '@lucide/svelte/icons/panel-right';
	import FolderIcon from '@lucide/svelte/icons/folder';
	import { Folder, FolderTree } from '@lucide/svelte';
	import { filesize } from 'filesize';
	import { cn } from '$lib/utils';
	import * as Breadcrumb from '$lib/components/ui/breadcrumb';
	import { useFileManager } from './file-manager.svelte';
	import { PressedKeys, watch } from 'runed';
	import type { TreeDirectory } from '$lib/api_bindings/files/TreeDirectory';
	import type { TreeFile } from '$lib/api_bindings/files/TreeFile';

	const fm = useFileManager();

	const recursivePath = $derived(
		fm.currentPath
			.split('/')
			.filter((s) => s)
			.reduce(
				(pre, name) => [...pre, { href: `${pre.at(-1)?.href ?? ''}/${name}`, name }],
				[] as { href: string; name: string }[]
			)
	);

	const keys = new PressedKeys();
	const items = $derived([...fm.currentSubDirectories, ...fm.currentFiles]);
	const isCtrlPressed = $derived(keys.has('Control'));
	const isShiftPressed = $derived(keys.has('Shift'));

	// Move this selection logic into fileManager context?
	let lastIndex = $state<number | undefined>(undefined);
	watch(
		() => items,
		() => {
			lastIndex = undefined;
		}
	);
	function selectItem(item: TreeDirectory | TreeFile, index: number) {
		if (isShiftPressed) {
			if (lastIndex !== undefined) {
				items
					.slice(Math.min(index, lastIndex), Math.max(index, lastIndex) + 1)
					.forEach((i) => fm.addSelected(i));
			} else {
				fm.toggleSelected(item);
				lastIndex = index;
			}
		} else if (isCtrlPressed) {
			fm.toggleSelected(item);
			lastIndex = index;
		} else {
			fm.setSelection(item);
			lastIndex = index;
		}
	}

	function clearSelection(
		e: MouseEvent & {
			currentTarget: EventTarget & HTMLDivElement;
		}
	) {
		if (e.target === e.currentTarget) {
			fm.clearSelection();
			lastIndex = undefined;
		}
	}
</script>

<div class="flex-1 flex flex-col min-w-0 bg-background">
	<header class="flex items-center justify-between px-4 py-3 border-b border-border">
		<div class="flex items-center gap-2">
			<Button
				variant="ghost"
				size="icon"
				onclick={() => fm.toggleSidebar()}
				class={fm.sidebarOpen
					? 'bg-primary/10 text-primary'
					: 'text-muted-foreground hover:text-foreground'}
			>
				<FolderTree />
			</Button>

			<Separator orientation="vertical" class="h-4 mx-2" />
			<Breadcrumb.Root>
				<Breadcrumb.List>
					{#each [{ name: 'Home', href: '/' }, ...recursivePath] as dir, i}
						<Breadcrumb.Item>
							{#if i === recursivePath.length}
								<Breadcrumb.Page>
									{dir.name}
								</Breadcrumb.Page>
							{:else}
								<Breadcrumb.Link class="cursor-pointer" onclick={() => fm.navigate(dir.href)}>
									{dir.name}
								</Breadcrumb.Link>
								<Breadcrumb.Separator />
							{/if}
						</Breadcrumb.Item>
					{/each}
				</Breadcrumb.List>
			</Breadcrumb.Root>
		</div>

		<div class="flex items-center gap-2">
			<div class="flex items-center bg-muted rounded-md p-0.5">
				<Button
					variant="ghost"
					size="sm"
					class="h-7 w-7 p-0 {fm.viewMode === 'grid'
						? 'bg-background text-foreground shadow-sm'
						: 'text-muted-foreground hover:text-foreground hover:bg-transparent'}"
					onclick={() => (fm.viewMode = 'grid')}
					aria-label="Grid view"
				>
					<LayoutGridIcon class="w-4 h-4" />
				</Button>
				<Button
					variant="ghost"
					size="sm"
					class="h-7 w-7 p-0 {fm.viewMode === 'list'
						? 'bg-background text-foreground shadow-sm'
						: 'text-muted-foreground hover:text-foreground hover:bg-transparent'}"
					onclick={() => (fm.viewMode = 'list')}
					aria-label="List view"
				>
					<ListIcon class="w-4 h-4" />
				</Button>
			</div>

			<Button
				variant="ghost"
				size="sm"
				class="h-8 w-8 p-0 {fm.previewOpen
					? 'bg-primary/10 text-primary'
					: 'text-muted-foreground hover:text-foreground'}"
				onclick={() => fm.togglePreview()}
				aria-label="Toggle preview panel"
			>
				<PanelRightIcon class="w-4 h-4" />
			</Button>
		</div>
	</header>

	<ScrollArea class="flex-1">
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<div class="p-4 h-full" onclick={clearSelection}>
			{#if fm.currentEmpty()}
				<div class="flex flex-col items-center justify-center h-64 text-muted-foreground">
					<FolderIcon class="w-16 h-16 mb-4 stroke-1" />
					<p>This folder is empty</p>
				</div>
			{:else if fm.viewMode === 'grid'}
				{@const buttonClasses =
					'flex h-fit flex-col items-center p-4 rounded-lg border transition-all cursor-pointer'}
				<div
					class="grid grid-cols-[repeat(auto-fill,minmax(160px,1fr))] gap-3"
					onclick={clearSelection}
				>
					{#each items as item, i}
						{#if 'directories' in item}
							<button
								class={cn(
									buttonClasses,
									fm.isSelected(item)
										? 'border-primary bg-primary/5'
										: 'border-transparent hover:bg-muted'
								)}
								onclick={() => selectItem(item, i)}
								ondblclick={() => fm.navigate(item)}
							>
								<Folder class="w-12 h-12" />
								<span class="mt-2 text-sm text-foreground text-center truncate w-full"
									>{item.name}</span
								>
								<!-- <span class="text-xs text-muted-foreground">{filesize(file.size)}</span> -->
							</button>
						{:else}
							<button
								draggable="true"
								class={cn(
									buttonClasses,
									fm.isSelected(item)
										? 'border-primary bg-primary/5'
										: 'border-transparent hover:bg-muted'
								)}
								onclick={() => selectItem(item, i)}
							>
								<FileIcon extension={item.name.split('.').at(-1)} size="lg" />
								<span class="mt-2 text-sm text-foreground text-center break-all w-full">
									{item.name}
								</span>
								<span class="text-xs text-muted-foreground">{filesize(item.size)}</span>
							</button>
						{/if}
					{/each}
				</div>
			{:else}
				<div class="border border-border rounded-lg overflow-hidden">
					<table class="w-full">
						<thead>
							<tr class="bg-muted/50 text-left text-sm text-muted-foreground">
								<th class="px-4 py-3 font-medium">Name</th>
								<th class="px-4 py-3 font-medium w-32">Size</th>
								<th class="px-4 py-3 font-medium w-46">Modified</th>
							</tr>
						</thead>
						<tbody>
							{#each items as item, i}
								{#if 'directories' in item}
									<tr
										class="border-t border-border transition-colors cursor-pointer {fm.isSelected(
											item
										)
											? 'bg-primary/5'
											: 'hover:bg-muted/50'}"
										onclick={() => selectItem(item, i)}
										ondblclick={() => fm.navigate(item)}
										onkeydown={(e) => e.key === 'Enter' && selectItem(item, i)}
										tabindex="0"
										role="button"
									>
										<td class="px-4 py-3">
											<div class="flex items-center gap-3">
												<Folder class="w-5 h-5" />
												<span class="text-sm text-foreground">{item.name}</span>
											</div>
										</td>
										<td class="px-4 py-3 text-sm text-muted-foreground"
											>{item.directories.length} item(s)</td
										>
										<td class="px-4 py-3 text-sm text-muted-foreground"></td>
									</tr>
								{:else}
									<tr
										class="border-t border-border transition-colors cursor-pointer focus:outline-0 {fm.isSelected(
											item
										)
											? 'bg-primary/5'
											: 'hover:bg-muted/50'}"
										onclick={() => selectItem(item, i)}
										onkeydown={(e) => e.key === 'Enter' && selectItem(item, i)}
										tabindex="0"
										role="button"
									>
										<td class="px-4 py-3">
											<div class="flex items-center gap-3">
												<FileIcon extension={item.name.split('.').at(-1)} size="sm" />
												<span class="text-sm text-foreground">{item.name}</span>
											</div>
										</td>
										<td class="px-4 py-3 text-sm text-muted-foreground">{filesize(item.size)}</td>
										<td class="px-4 py-3 text-sm text-muted-foreground">
											{new Date(item.date).toLocaleString()}
										</td>
									</tr>
								{/if}
							{/each}
						</tbody>
					</table>
				</div>
			{/if}
		</div>
	</ScrollArea>
</div>
