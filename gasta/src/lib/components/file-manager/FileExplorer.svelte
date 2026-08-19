<script lang="ts">
	import FileIcon from './FileIcon.svelte';
	import { Button } from '$lib/components/ui/button';
	import { Separator } from '$lib/components/ui/separator';
	import LayoutGridIcon from '@lucide/svelte/icons/layout-grid';
	import ListIcon from '@lucide/svelte/icons/list';
	import PanelRightIcon from '@lucide/svelte/icons/panel-right';
	import FolderIcon from '@lucide/svelte/icons/folder';
	import { Folder, FolderTree, History, House, Search, SearchIcon } from '@lucide/svelte';
	import { filesize } from 'filesize';
	import { cn } from '$lib/utils';
	import * as Breadcrumb from '$lib/components/ui/breadcrumb';
	import * as ContextMenu from '../ui/context-menu';
	import { useFileManager } from './file-manager.svelte';
	import { PressedKeys, watch } from 'runed';
	import type { TreeDirectory, TreeFile } from '$lib/server/sasta_client';
	import { CurrentPathType } from './types';

	const fm = useFileManager();

	const recursivePath = $derived(
		fm.currentPath.type === CurrentPathType.Path
			? fm.currentPath.path
					.split('/')
					.filter((s) => s)
					.reduce(
						(pre, name) => [...pre, { href: `${pre.at(-1)?.href ?? ''}/${name}`, name }],
						[] as { href: string; name: string }[]
					)
			: []
	);

	const keys = new PressedKeys();
	const items = $derived.by(() => {
		if (fm.currentPath.type === CurrentPathType.Path) {
			return [...fm.currentSubDirectories, ...fm.currentFiles];
		} else if (fm.currentPath.type === CurrentPathType.Recent) {
			const files: TreeFile[] = [];
			function treeToArray(item: TreeDirectory) {
				item.directories.forEach((i) => treeToArray(i));
				item.files.forEach((i) => files.push(i));
			}
			treeToArray(fm.root);
			return files.toSorted((a, b) => new Date(b.date).getTime() - new Date(a.date).getTime());
		} else if (fm.currentPath.type === CurrentPathType.Search) {
			const items: (TreeFile | TreeDirectory)[] = [];
			const search = fm.currentPath.search.trim().toLowerCase();
			function treeToArray(item: TreeDirectory) {
				item.directories.forEach((i) => {
					items.push(i);
					treeToArray(i);
				});
				item.files.forEach((i) => items.push(i));
			}
			treeToArray(fm.root);
			return items.filter((f) => f.name.toLowerCase().includes(search));
		}
		throw Error('Not Implemented');
	});
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

	watch(
		() => keys.all,
		() => {
			const active = document.activeElement;
			const isTyping = active instanceof HTMLInputElement || active instanceof HTMLTextAreaElement;

			const isDialogOpen = document.querySelector('[role="alertdialog"]') !== null;

			if (isTyping || isDialogOpen) {
				return;
			}

			if (keys.has('Control', 'X')) {
				fm.setClipboard(fm.getSelected(), 'clip');
			} else if (keys.has('Delete')) {
				fm.deleteFile(fm.getSelected());
			} else if (keys.has('Control', 'V') && fm.currentPath.type == CurrentPathType.Path) {
				fm.moveItems(fm.getClipboard(), fm.currentPath.path);
			}
		}
	);
</script>

<ContextMenu.Root>
	<ContextMenu.Trigger class="flex-1 flex flex-col min-w-0 min-h-0">
		{@render fileExplorer()}
	</ContextMenu.Trigger>
	<ContextMenu.Content class="w-52">
		<ContextMenu.Item
			inset
			onclick={() => fm.setClipboard(fm.getSelected(), 'clip')}
			disabled={fm.nbrSelected() < 1}
		>
			Cut
			{#if fm.nbrSelected() > 1}
				<i class="text-zinc-500">{fm.nbrSelected()} items</i>
			{/if}
			<ContextMenu.Shortcut>Ctrl+X</ContextMenu.Shortcut>
		</ContextMenu.Item>
		<ContextMenu.Item
			inset
			disabled={fm.clipboardEmpty || fm.currentPath.type != CurrentPathType.Path}
			onclick={() =>
				fm.currentPath.type == CurrentPathType.Path &&
				fm.moveItems(fm.getClipboard(), fm.currentPath.path)}
		>
			Paste
			{#if !fm.clipboardEmpty}
				<i class="text-zinc-500">{fm.clipboardSize} item(s)</i>
			{/if}
			<ContextMenu.Shortcut>Ctrl+V</ContextMenu.Shortcut>
		</ContextMenu.Item>
		<ContextMenu.Item
			inset
			onclick={() => fm.deleteFile(fm.getSelected())}
			disabled={fm.nbrSelected() < 1}
		>
			Delete
			{#if fm.nbrSelected() > 1}
				<i class="text-zinc-500">{fm.nbrSelected()} items</i>
			{/if}
			<ContextMenu.Shortcut>delete</ContextMenu.Shortcut>
		</ContextMenu.Item>
	</ContextMenu.Content>
</ContextMenu.Root>

{#snippet fileExplorer()}
	<div class="flex-1 flex flex-col min-w-0 min-h-0 bg-background">
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
						<Breadcrumb.Item>
							{#if fm.currentPath.type === CurrentPathType.Path}
								{#if recursivePath?.length}
									<Breadcrumb.Link
										class="cursor-pointer inline-flex items-center gap-1"
										onclick={() => fm.navigate('/')}
									>
										<House size={18} />
									</Breadcrumb.Link>
									<Breadcrumb.Separator />
								{:else}
									<Breadcrumb.Page class="inline-flex items-center gap-1">
										<House size={18} />
									</Breadcrumb.Page>
								{/if}
							{:else if fm.currentPath.type === CurrentPathType.Search}
								<Breadcrumb.Page><Search size={18} /></Breadcrumb.Page>
							{:else}
								<Breadcrumb.Page><History size={18} /></Breadcrumb.Page>
							{/if}
						</Breadcrumb.Item>
						{#if recursivePath}
							{#each recursivePath as dir, i}
								<Breadcrumb.Item>
									{#if i === recursivePath.length - 1}
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
						{/if}
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

		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<div class="p-4 flex-1 overflow-y-auto box-border" onclick={clearSelection}>
			{#if !items.length}
				<div class="flex flex-col items-center justify-center h-64 text-muted-foreground">
					{#if fm.currentPath.type == CurrentPathType.Search}
						<SearchIcon class="w-16 h-16 mb-4 stroke-1" />
						<p>No Items matched search</p>
					{:else}
						<FolderIcon class="w-16 h-16 mb-4 stroke-1" />
						<p>This folder is empty</p>
					{/if}
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
										: 'border-transparent hover:bg-muted',
									fm.isInClipboard(item) && 'opacity-40'
								)}
								onclick={() => selectItem(item, i)}
								ondblclick={() => fm.navigate(item)}
							>
								<Folder class="w-12 h-12" />
								<span class="mt-2 text-sm text-foreground text-center truncate w-full">
									{item.name}
								</span>
								<!-- <span class="text-xs text-muted-foreground">{filesize(file.size)}</span> -->
							</button>
						{:else}
							<button
								draggable="true"
								class={cn(
									buttonClasses,
									fm.isSelected(item)
										? 'border-primary bg-primary/5'
										: 'border-transparent hover:bg-muted',
									fm.isInClipboard(item) && 'opacity-40'
								)}
								onclick={() => selectItem(item, i)}
								ondblclick={() => {
									if (fm.currentPath.type === CurrentPathType.Path) {
										fm.previewOpen = true;
									} else {
										fm.navigate(item.id.replace(item.name, ''));
										fm.setSelection(item);
										fm.previewOpen = true;
									}
								}}
							>
								<FileIcon extension={item.name.split('.').at(-1)} size="lg" />
								<span class="mt-2 text-sm text-foreground text-center break-all w-full">
									{item.name}
								</span>
								<span class="text-xs text-muted-foreground">
									{#if fm.currentPath.type === CurrentPathType.Recent}
										{new Date(item.date).toLocaleString()}
									{:else}
										{filesize(item.size)}
									{/if}
								</span>
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
	</div>
{/snippet}
