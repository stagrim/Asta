<script lang="ts">
	import FileIcon from './FileIcon.svelte';
	import { Button } from '$lib/components/ui/button';
	import { ScrollArea } from '$lib/components/ui/scroll-area';
	import { Separator } from '$lib/components/ui/separator';
	import DownloadIcon from '@lucide/svelte/icons/download';
	import PencilIcon from '@lucide/svelte/icons/pencil';
	import Trash2Icon from '@lucide/svelte/icons/trash-2';
	import EyeIcon from '@lucide/svelte/icons/eye';
	import { filesize } from 'filesize';
	import { Files, Folder } from '@lucide/svelte';
	import * as Resizable from '$lib/components/ui/resizable';
	import { watch } from 'runed';
	import * as Sheet from '$lib/components/ui/sheet';
	import { useFileManager } from './file-manager.svelte';
	import type { TreeFile } from '$lib/server/sasta_client';

	// svelte-ignore non_reactive_update
	let pane: ReturnType<typeof Resizable.Pane>;

	const fm = useFileManager();

	watch(
		() => fm.previewOpen,
		() => {
			if (pane) {
				if (fm.previewOpen && pane.isCollapsed()) {
					pane.expand();
				} else if (!fm.previewOpen && pane.isExpanded()) {
					pane.collapse();
				}
			}
		}
	);

	function previews(file: TreeFile) {
		const authorized_extensions = ['jpg', 'jpeg', 'png', 'webp', 'gif', 'svg'];
		// TODO: break out into a function
		const ext = file.name.split('.').at(-1)!;
		return authorized_extensions.includes(ext) ? `/files/${file.id}` : false;
	}
</script>

{#snippet previewContent()}
	{@const selectedItem = fm.oneSelected()}
	<aside class="w-full h-full shrink-0 border-l border-border bg-card flex flex-col">
		<header class="flex items-center justify-between px-4 py-3 border-b border-border">
			<h2 class="font-medium text-foreground">Preview</h2>
		</header>

		{#if selectedItem}
			<ScrollArea class="flex-1">
				<div class="p-4">
					<div class="flex flex-col items-center mb-6">
						<div class="rounded-lg bg-muted flex items-center justify-center mb-3">
							<!-- TODO: Make this a util function? -->
							<!-- Check if a TreeDirectory -->
							{#if 'directories' in selectedItem}
								<Folder class="w-12 h-12" />
							{:else}
								{@const previewURL = previews(selectedItem)}
								{#if previewURL}
									<img class="mb-4" src={previewURL} alt="" />
								{:else}
									<FileIcon extension="{selectedItem.name.split('.').at(-1)}}" size="lg" />
								{/if}
							{/if}
						</div>
						<h3 class="text-sm font-medium text-foreground text-center break-all">
							{selectedItem.name}
						</h3>
						<!-- <p class="text-xs text-muted-foreground mt-1">{file.type.toUpperCase()}</p> -->
					</div>

					<Separator class="my-4" />

					<div class="space-y-4">
						{#if 'size' in selectedItem}
							<div>
								<h4 class="text-xs font-medium text-muted-foreground uppercase tracking-wider mb-2">
									Details
								</h4>
								<dl class="space-y-2">
									<div class="flex justify-between">
										<dt class="text-sm text-muted-foreground">Size</dt>
										<dd class="text-sm text-foreground">{filesize(selectedItem.size)}</dd>
									</div>
									<div class="flex justify-between">
										<dt class="text-sm text-muted-foreground">Modified</dt>
										<dd class="text-sm text-foreground">
											{new Date(selectedItem.date).toLocaleString()}
										</dd>
									</div>
								</dl>
							</div>
						{:else}
							<div>
								<h4 class="text-xs font-medium text-muted-foreground uppercase tracking-wider mb-2">
									Details
								</h4>
								<dl class="space-y-2">
									<div class="flex justify-between">
										<dt class="text-sm text-muted-foreground">Size</dt>
										<dd class="text-sm text-foreground">
											{selectedItem.directories.length + selectedItem.files.length} item(s)
										</dd>
									</div>
								</dl>
							</div>
						{/if}

						<Separator />

						<div>
							<h4 class="text-xs font-medium text-muted-foreground uppercase tracking-wider mb-2">
								Actions
							</h4>
							<div class="grid grid-cols-2 gap-2">
								<Button variant="secondary" size="sm" class="gap-2">
									<DownloadIcon class="w-4 h-4" />
									Download
								</Button>
								<Button variant="secondary" size="sm" class="gap-2">
									<PencilIcon class="w-4 h-4" />
									Rename
								</Button>
								<Button variant="secondary" size="sm" class="gap-2">
									<Folder class="w-4 h-4" />
									Move
								</Button>
								<Button variant="destructive" size="sm" class="gap-2">
									<Trash2Icon class="w-4 h-4" />
									Delete
								</Button>
							</div>
						</div>
					</div>
				</div>
			</ScrollArea>
		{:else if fm.nbrSelected() > 1}
			<div class="flex-1 flex flex-col items-center justify-center text-muted-foreground p-4">
				<Files class="w-12 h-12 mb-3 stroke-1" />
				<p class="text-sm text-center">{fm.nbrSelected()} items selected</p>
			</div>
		{:else}
			<div class="flex-1 flex flex-col items-center justify-center text-muted-foreground p-4">
				<EyeIcon class="w-12 h-12 mb-3 stroke-1" />
				<p class="text-sm text-center">Select a file to preview</p>
			</div>
		{/if}
	</aside>
{/snippet}

{#if fm.isMobile}
	<Sheet.Root bind:open={fm.previewOpen}>
		<Sheet.Content side="right" class="p-0 w-[300px]">
			{@render previewContent()}
		</Sheet.Content>
	</Sheet.Root>
{:else}
	<Resizable.Pane
		bind:this={pane}
		collapsible={true}
		onCollapse={() => (fm.previewOpen = false)}
		onExpand={() => (fm.previewOpen = true)}
		defaultSize={0}
		minSize={15}
		maxSize={40}
	>
		{#if fm.previewOpen}
			{@render previewContent()}
		{/if}
	</Resizable.Pane>
{/if}
