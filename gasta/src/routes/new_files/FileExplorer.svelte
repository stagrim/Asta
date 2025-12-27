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

	const fm = useFileManager();

	let recursivePath = $derived(
		fm.currentPath
			.split('/')
			.filter((s) => s)
			.reduce(
				(pre, cur, i) => [...pre, { href: `${pre.at(-1)?.href ?? ''}/${cur}`, name: cur }],
				[] as { href: string; name: string }[]
			)
	);
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
		<div
			class="p-4 h-full"
			onclick={(e) => {
				if (e.target === e.currentTarget) {
					fm.selectedItem = null;
				}
			}}
		>
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
					onclick={(e) => {
						if (e.target === e.currentTarget) {
							fm.selectedItem = null;
						}
					}}
				>
					{#each fm.currentSubDirectories as dir}
						<button
							class={cn(
								buttonClasses,
								fm.selectedItem?.name === dir.name
									? 'border-primary bg-primary/5'
									: 'border-transparent hover:bg-muted'
							)}
							onclick={() => (fm.selectedItem = dir)}
							ondblclick={() => fm.navigate(dir)}
						>
							<Folder class="w-12 h-12" />
							<span class="mt-2 text-sm text-foreground text-center truncate w-full"
								>{dir.name}</span
							>
							<!-- <span class="text-xs text-muted-foreground">{filesize(file.size)}</span> -->
						</button>
					{/each}
					{#each fm.currentFiles as file}
						<button
							draggable="true"
							class={cn(
								buttonClasses,
								fm.selectedItem?.name === file.name
									? 'border-primary bg-primary/5'
									: 'border-transparent hover:bg-muted'
							)}
							onclick={() => (fm.selectedItem = file)}
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
							{#each fm.currentSubDirectories as directory}
								<tr
									class="border-t border-border transition-colors cursor-pointer {fm.selectedItem
										?.name === directory.name
										? 'bg-primary/5'
										: 'hover:bg-muted/50'}"
									onclick={() => (fm.selectedItem = directory)}
									onkeydown={(e) => e.key === 'Enter' && (fm.selectedItem = directory)}
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
							{#each fm.currentFiles as file}
								<tr
									class="border-t border-border transition-colors cursor-pointer {fm.selectedItem
										?.name === file.name
										? 'bg-primary/5'
										: 'hover:bg-muted/50'}"
									onclick={() => (fm.selectedItem = file)}
									onkeydown={(e) => e.key === 'Enter' && (fm.selectedItem = file)}
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
