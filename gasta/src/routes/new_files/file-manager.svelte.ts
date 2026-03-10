import { setContext, getContext } from 'svelte';
import { IsMobile } from '$lib/hooks/is-mobile.svelte';
import { SvelteSet } from 'svelte/reactivity';
import type { TreeDirectory, TreeFile } from '$lib/server/sasta_client';
import type { FileManagerAPI } from './types';

const FM_KEY = Symbol('FILE_MANAGER');

export class FileManager {
	#root;
	/** Get root tree node */
	get root() {
		return this.#root;
	}

	async refresh() {
		try {
			this.#root = await this.#api.getFileTree();

			// Sync current path to the new tree
			if (this.#currentPath === '/') {
				this.#currentDirectory = this.#root;
			} else {
				// Find the new object reference for the folder we are currently in
				const updatedCurrentDir = this.#traverseTree(this.#currentPath);

				if (updatedCurrentDir) {
					this.#currentDirectory = updatedCurrentDir;
				} else {
					// Fallback to root if the folder was deleted!
					this.#currentPath = '/';
					this.#currentDirectory = this.#root;
				}
			}
		} catch (e) {
			console.error('Failed to refresh tree: ', e);
		}
	}

	// Current active path and directory
	#currentPath = $state('/');
	/** String of currently opened directory's path */
	get currentPath() {
		return this.#currentPath;
	}

	#isMobile: IsMobile = new IsMobile();

	#currentDirectory;
	/** Object of currently active Directory */
	get currentDirectory() {
		return this.#currentDirectory;
	}

	// Selection & UI
	#selectedItem = new SvelteSet<TreeFile | TreeDirectory>();

	/** Returns wether the given item is selected or not */
	isSelected(item: TreeFile | TreeDirectory): boolean {
		return this.#selectedItem.has(item);
	}

	/** Set the current item as the only selected item */
	setSelection(item: TreeFile | TreeDirectory) {
		this.clearSelection();
		this.addSelected(item);
	}

	/** Clear all selected items */
	clearSelection() {
		this.#selectedItem.clear();
	}

	/** Select the given item */
	addSelected(item: TreeFile | TreeDirectory) {
		this.#selectedItem.add(item);
	}

	/** Deselect the given item */
	removeSelected(item: TreeFile | TreeDirectory) {
		this.#selectedItem.delete(item);
	}

	/** Toggle select state of the given item */
	toggleSelected(item: TreeFile | TreeDirectory) {
		this.#selectedItem.has(item) ? this.#selectedItem.delete(item) : this.#selectedItem.add(item);
	}

	/** Number of selected items */
	nbrSelected() {
		return this.#selectedItem.size;
	}

	/** Gives the selected `TreeDirectory` or `TreeFile` object if it is the only selected item. Returns null if more or less than one is selected */
	oneSelected(): TreeDirectory | TreeFile | null {
		return this.nbrSelected() == 1 ? this.#selectedItem.values().next().value! : null;
	}

	getSelected(): (TreeDirectory | TreeFile)[] {
		return [...this.#selectedItem.values()];
	}

	#clipboardMode: 'copy' | 'clip' = 'copy';
	#clipboard = new SvelteSet<TreeFile | TreeDirectory>();
	/** Empties, and sets `items` to the current clipboard content */
	setClipboard(items: (TreeFile | TreeDirectory)[], mode: 'copy' | 'clip') {
		this.#clipboard.clear();
		items.forEach((i) => this.#clipboard.add(i));
		this.#clipboardMode = mode;
	}

	isInClipboard(item: TreeFile | TreeDirectory): boolean {
		return this.#clipboard.has(item);
	}

	get clipboardMode() {
		return this.#clipboardMode;
	}

	// Layout State
	viewMode = $state<'grid' | 'list'>('grid');
	sidebarOpen = $state(!this.#isMobile.current);
	previewOpen = $state(false);

	#api: FileManagerAPI;

	async deleteFile(ids: (TreeFile | TreeDirectory)[]): Promise<boolean> {
		try {
			await this.#api.deleteFile(ids.map((t) => t.id));
		} catch (e) {
			console.error(e);
			await this.refresh();
			return false;
		}
		ids.forEach((t) => this.#selectedItem.delete(t));
		await this.refresh();
		return true;
	}

	constructor(api: FileManagerAPI, initialRoot: TreeDirectory) {
		this.#api = api;
		this.#root = $state(initialRoot);
		this.#currentDirectory = $state(this.#root);
	}

	/** Get the files in the currently active Directory */
	get currentFiles() {
		return this.#currentDirectory?.files ?? [];
	}

	/** Get the direct subdirectories of the currently active Directory */
	get currentSubDirectories() {
		return this.#currentDirectory?.directories ?? [];
	}

	/** If the current Directory is empty */
	currentEmpty() {
		return this.currentFiles.length == 0 && this.currentSubDirectories.length == 0;
	}

	/** Change currently active directory, either by string path, or by a `TreeDirectory` object */
	navigate(directory: TreeDirectory | string) {
		if (typeof directory === 'string') {
			const dir = this.#traverseTree(directory);
			if (dir) {
				this.#currentPath = dir.id;
				this.#currentDirectory = dir;
				this.clearSelection();
			} else {
				console.error(`${directory} was not found`);
			}
		} else {
			this.#currentPath = directory.id;
			this.#currentDirectory = directory;
			this.clearSelection();
		}
	}

	#traverseTree(path: string): TreeDirectory | null {
		const dirs = path.split('/').filter((s) => s);

		if (dirs.length === 0) {
			return this.#root;
		}

		let dir = this.#root;
		for (const x of dirs) {
			const res = dir.directories.find((d) => d.name === x);
			if (res) {
				dir = res;
			} else {
				return null;
			}
		}
		return dir;
	}

	/** Convenience getter for checking if the panels are in mobile mode */
	get isMobile() {
		return this.#isMobile.current;
	}

	/** Toggles the sidebar open/closed */
	toggleSidebar() {
		this.sidebarOpen = !this.sidebarOpen;
	}

	/** Toggles the preview panel open/closed */
	togglePreview() {
		this.previewOpen = !this.previewOpen;
	}
}

/**
 * Instantiates a new `FileManager` instance and sets it in the context.
 *
 * @param api implementation of the `FileManagerAPI` interface for backend communication
 * @param root The root TreeDirectory node of the file system
 * @returns  The `FileManager` instance.
 */
export function createFileManager(api: FileManagerAPI, root: TreeDirectory): FileManager {
	return setContext(FM_KEY, new FileManager(api, root));
}

/**
 * Retrieves the `FileManager` instance from the context. This is a class instance,
 * so you cannot destructure it.
 * @returns The `FileManager` instance.
 */
export function useFileManager() {
	return getContext<FileManager>(FM_KEY);
}
