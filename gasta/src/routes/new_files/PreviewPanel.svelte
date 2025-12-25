<script lang="ts">
	import FileIcon from './FileIcon.svelte';
	import { Button } from '$lib/components/ui/button';
	import { ScrollArea } from '$lib/components/ui/scroll-area';
	import { Separator } from '$lib/components/ui/separator';
	import DownloadIcon from '@lucide/svelte/icons/download';
	import PencilIcon from '@lucide/svelte/icons/pencil';
	import Trash2Icon from '@lucide/svelte/icons/trash-2';
	import EyeIcon from '@lucide/svelte/icons/eye';
	import type { TreeFile } from '$lib/api_bindings/files/TreeFile';
	import { filesize } from 'filesize';
	import { Folder } from '@lucide/svelte';
	import type { TreeDirectory } from '$lib/api_bindings/files/TreeDirectory';
	import * as Resizable from '$lib/components/ui/resizable';
	import { IsMobile } from '$lib/hooks/is-mobile.svelte';
	import { watch } from 'runed';
	import * as Sheet from '$lib/components/ui/sheet';

	const isMobile = new IsMobile();

	let sheetOpen = $state<boolean>(false);
	// svelte-ignore non_reactive_update
	let pane: ReturnType<typeof Resizable.Pane>;

	let {
		file: item,
		open = $bindable()
	}: {
		file: TreeFile | TreeDirectory | null;
		open?: boolean;
	} = $props();

	watch(
		() => open,
		() => {
			if (open !== undefined) {
				sheetOpen = open;
			}

			if (pane) {
				if (open && pane.isCollapsed()) {
					pane.expand();
				} else if (!open && pane.isExpanded()) {
					pane.collapse();
				}
			}
		}
	);

	watch(
		() => sheetOpen,
		() => {
			open = sheetOpen;
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
	<aside class="w-full h-full shrink-0 border-l border-border bg-card flex flex-col">
		<header class="flex items-center justify-between px-4 py-3 border-b border-border">
			<h2 class="font-medium text-foreground">Preview</h2>
		</header>

		{#if item}
			<ScrollArea class="flex-1">
				<div class="p-4">
					<div class="flex flex-col items-center mb-6">
						<div class="rounded-lg bg-muted flex items-center justify-center mb-3">
							{#if 'directories' in item}
								<Folder class="w-12 h-12" />
							{:else}
								{@const previewURL = previews(item)}
								{#if previewURL}
									<img class="mb-4" src={previewURL} alt="" />
								{:else}
									<FileIcon extension="{item.name.split('.').at(-1)}}" size="lg" />
								{/if}
							{/if}
						</div>
						<h3 class="text-sm font-medium text-foreground text-center break-all">{item.name}</h3>
						<!-- <p class="text-xs text-muted-foreground mt-1">{file.type.toUpperCase()}</p> -->
					</div>

					<Separator class="my-4" />

					<div class="space-y-4">
						{#if 'size' in item}
							<div>
								<h4 class="text-xs font-medium text-muted-foreground uppercase tracking-wider mb-2">
									Details
								</h4>
								<dl class="space-y-2">
									<div class="flex justify-between">
										<dt class="text-sm text-muted-foreground">Size</dt>
										<dd class="text-sm text-foreground">{filesize(item.size)}</dd>
									</div>
									<div class="flex justify-between">
										<dt class="text-sm text-muted-foreground">Modified</dt>
										<dd class="text-sm text-foreground">{new Date(item.date).toLocaleString()}</dd>
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
										<dd class="text-sm text-foreground">{item.directories.length} item(s)</dd>
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
		{:else}
			<div class="flex-1 flex flex-col items-center justify-center text-muted-foreground p-4">
				<EyeIcon class="w-12 h-12 mb-3 stroke-1" />
				<p class="text-sm text-center">Select a file to preview</p>
			</div>
		{/if}
	</aside>
{/snippet}

{#if isMobile.current}
	<Sheet.Root bind:open={sheetOpen}>
		<Sheet.Content side="right" class="p-0 w-[300px]">
			{@render previewContent()}
		</Sheet.Content>
	</Sheet.Root>
{:else}
	<Resizable.Pane bind:this={pane} collapsible={true} defaultSize={0} minSize={15} maxSize={40}>
		{@render previewContent()}
	</Resizable.Pane>
{/if}
