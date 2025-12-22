<script lang="ts">
	import FileIcon from './FileIcon.svelte';
	import { Trigger as SidebarTrigger, useSidebar } from '$lib/components/ui/sidebar';
	import { Button } from '$lib/components/ui/button';
	import { ScrollArea } from '$lib/components/ui/scroll-area';
	import { Separator } from '$lib/components/ui/separator';
	import LayoutGridIcon from '@lucide/svelte/icons/layout-grid';
	import ListIcon from '@lucide/svelte/icons/list';
	import PanelRightIcon from '@lucide/svelte/icons/panel-right';
	import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
	import FolderIcon from '@lucide/svelte/icons/folder';
	import { Folder, FolderTree } from '@lucide/svelte';
	import type { TreeDirectory } from '$lib/api_bindings/files/TreeDirectory';
	import type { TreeFile } from '$lib/api_bindings/files/TreeFile';
	import { filesize } from 'filesize';
	import { cn } from '$lib/utils';

	let {
		files,
		directories,
		selectedItem,
		selectedFolder,
		viewMode,
		showPreview,
		onFileSelect,
		onFolderSelect,
		onTogglePreview,
		onViewModeChange
	}: {
		files: TreeFile[];
		directories: TreeDirectory[];
		selectedItem: TreeFile | TreeDirectory | null;
		selectedFolder: string;
		viewMode: 'grid' | 'list';
		showPreview: boolean;
		onFileSelect: (file: TreeFile | TreeDirectory | null) => void;
		onFolderSelect: (folder: string | TreeDirectory) => void;
		onTogglePreview: () => void;
		onViewModeChange: (mode: 'grid' | 'list') => void;
	} = $props();

	const sidebar = useSidebar();
</script>

<div class="flex-1 flex flex-col min-w-0 bg-background">
	<header class="flex items-center justify-between px-4 py-3 border-b border-border">
		<div class="flex items-center gap-2">
			<Button variant="ghost" size="icon" onclick={() => sidebar.toggle()}>
				<FolderTree />
			</Button>
			<Separator orientation="vertical" class="h-4 mx-2" />
			<nav class="flex items-center gap-1 text-sm">
				<span class="text-muted-foreground">Home</span>
				{#each selectedFolder.split('/').filter((s) => s) as dir}
					<ChevronRightIcon class="w-4 h-4 text-muted-foreground" />
					<span class="text-foreground font-medium">{dir}</span>
				{/each}
			</nav>
		</div>

		<div class="flex items-center gap-2">
			<div class="flex items-center bg-muted rounded-md p-0.5">
				<Button
					variant="ghost"
					size="sm"
					class="h-7 w-7 p-0 {viewMode === 'grid'
						? 'bg-background text-foreground shadow-sm'
						: 'text-muted-foreground hover:text-foreground hover:bg-transparent'}"
					onclick={() => onViewModeChange('grid')}
					aria-label="Grid view"
				>
					<LayoutGridIcon class="w-4 h-4" />
				</Button>
				<Button
					variant="ghost"
					size="sm"
					class="h-7 w-7 p-0 {viewMode === 'list'
						? 'bg-background text-foreground shadow-sm'
						: 'text-muted-foreground hover:text-foreground hover:bg-transparent'}"
					onclick={() => onViewModeChange('list')}
					aria-label="List view"
				>
					<ListIcon class="w-4 h-4" />
				</Button>
			</div>

			<Button
				variant="ghost"
				size="sm"
				class="h-8 w-8 p-0 {showPreview
					? 'bg-primary/10 text-primary'
					: 'text-muted-foreground hover:text-foreground'}"
				onclick={onTogglePreview}
				aria-label="Toggle preview panel"
			>
				<PanelRightIcon class="w-4 h-4" />
			</Button>
		</div>
	</header>

	<ScrollArea class="flex-1">
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<div
			class="p-4 h-full"
			onclick={(e) => {
				if (e.target === e.currentTarget) {
					onFileSelect(null);
				}
			}}
		>
			{#if files.length === 0 && directories.length === 0}
				<div class="flex flex-col items-center justify-center h-64 text-muted-foreground">
					<FolderIcon class="w-16 h-16 mb-4 stroke-1" />
					<p>This folder is empty</p>
				</div>
			{:else if viewMode === 'grid'}
				{@const buttonClasses =
					'flex flex-col items-center p-4 rounded-lg border transition-all cursor-pointer'}
				<div
					class="grid grid-cols-[repeat(auto-fill,minmax(160px,1fr))] gap-3"
					onclick={(e) => {
						if (e.target === e.currentTarget) {
							onFileSelect(null);
						}
					}}
				>
					{#each directories as dir}
						<button
							class={cn(
								buttonClasses,
								selectedItem?.name === dir.name
									? 'border-primary bg-primary/5'
									: 'border-transparent hover:bg-muted'
							)}
							onclick={() => onFileSelect(dir)}
							ondblclick={() => onFolderSelect(dir)}
						>
							<Folder class="w-12 h-12" />
							<span class="mt-2 text-sm text-foreground text-center truncate w-full"
								>{dir.name}</span
							>
							<!-- <span class="text-xs text-muted-foreground">{filesize(file.size)}</span> -->
						</button>
					{/each}
					{#each files as file}
						<button
							draggable="true"
							class={cn(
								buttonClasses,
								selectedItem?.name === file.name
									? 'border-primary bg-primary/5'
									: 'border-transparent hover:bg-muted'
							)}
							onclick={() => onFileSelect(file)}
						>
							<FileIcon extension={file.name.split('.').at(-1)} size="lg" />
							<span class="mt-2 text-sm text-foreground text-center break-all w-full">
								{file.name}
							</span>
							<span class="text-xs text-muted-foreground">{filesize(file.size)}</span>
						</button>
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
							{#each directories as directory}
								<tr
									class="border-t border-border transition-colors cursor-pointer {selectedItem?.name ===
									directory.name
										? 'bg-primary/5'
										: 'hover:bg-muted/50'}"
									onclick={() => onFileSelect(directory)}
									onkeydown={(e) => e.key === 'Enter' && onFileSelect(directory)}
									tabindex="0"
									role="button"
								>
									<td class="px-4 py-3">
										<div class="flex items-center gap-3">
											<Folder class="w-5 h-5" />
											<span class="text-sm text-foreground">{directory.name}</span>
										</div>
									</td>
									<td class="px-4 py-3 text-sm text-muted-foreground"
										>{directory.directories.length} item(s)</td
									>
									<td class="px-4 py-3 text-sm text-muted-foreground"></td>
								</tr>
							{/each}
							{#each files as file}
								<tr
									class="border-t border-border transition-colors cursor-pointer {selectedItem?.name ===
									file.name
										? 'bg-primary/5'
										: 'hover:bg-muted/50'}"
									onclick={() => onFileSelect(file)}
									onkeydown={(e) => e.key === 'Enter' && onFileSelect(file)}
									tabindex="0"
									role="button"
								>
									<td class="px-4 py-3">
										<div class="flex items-center gap-3">
											<FileIcon extension={file.name.split('.').at(-1)} size="sm" />
											<span class="text-sm text-foreground">{file.name}</span>
										</div>
									</td>
									<td class="px-4 py-3 text-sm text-muted-foreground">{filesize(file.size)}</td>
									<td class="px-4 py-3 text-sm text-muted-foreground">
										{new Date(file.date).toLocaleString()}
									</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			{/if}
		</div>
	</ScrollArea>
</div>
