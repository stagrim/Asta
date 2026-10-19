import { setContext, getContext } from 'svelte';
import { IsMobile } from '#lib/hooks/is-mobile.svelte.js';
import { SvelteSet } from 'svelte/reactivity';
import type { TreeDirectory, TreeFile } from '#lib/server/sasta_client/index.ts';
import {
	CurrentPathType,
	FileManagerSearchParams,
	fileTypeFilters,
	type FileManagerAPI,
	type FileTypeFilterObject,
	type ValidFileTypeFilters
} from './types';
import { fileExtensionType } from './utils';
import { goto } from '$app/navigation';
import { page } from '$app/state';
import * as v from 'valibot';

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

			if (this.currentPath.type === CurrentPathType.Path) {
				if (!this.#traverseTree(this.currentPath.path)) {
					this.#setParams({ path: '/' }, true);
				}
			}
		} catch (e) {
			console.error('Failed to refresh tree: ', e);
		}
	}

	#params = $derived(v.parse(FileManagerSearchParams, Object.fromEntries(page.url.searchParams)));

	#setParams(updates: Partial<v.InferOutput<typeof FileManagerSearchParams>>, replace = false) {
		const next = v.parse(FileManagerSearchParams, { ...this.#params, ...updates });
		this.#params = next;

		const defaults = v.getDefaults(FileManagerSearchParams);
		const searchParams = new URLSearchParams(page.url.searchParams.toString());

		for (const [key, value] of Object.entries(next) as [keyof typeof defaults, string][]) {
			if (!value || value === defaults[key]) {
				searchParams.delete(key);
			} else {
				searchParams.set(key, value);
			}
		}

		const query = searchParams.toString();
		goto(query ? `?${query}` : '?', { replace, reset: false });
	}

	/** Computed property: Always reflects the current URL parameters */
	get currentPath():
		| {
				type: CurrentPathType.Search;
				search: string;
		  }
		| {
				type: CurrentPathType.Recent;
		  }
		| {
				type: CurrentPathType.Path;
				path: string;
		  } {
		if (this.#params.type === CurrentPathType.Search) {
			return { type: CurrentPathType.Search, search: this.#params.search };
		} else if (this.#params.type === CurrentPathType.Recent) {
			return { type: CurrentPathType.Recent };
		}
		return { type: CurrentPathType.Path, path: this.#params.path };
	}

	#isMobile: IsMobile = new IsMobile();

	/** Computed property: Finds the active directory based on the URL path */
	get currentDirectory() {
		if (this.currentPath.type === CurrentPathType.Path) {
			// Traverse the tree. If the URL points to a deleted/invalid folder, fallback to root.
			return this.#traverseTree(this.currentPath.path) || this.#root;
		}
		return this.#root;
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

	get clipboardEmpty() {
		return this.#clipboard.size == 0;
	}

	get clipboardSize() {
		return this.#clipboard.size;
	}

	/** Get items in clipboard */
	getClipboard(): (TreeFile | TreeDirectory)[] {
		return [...this.#clipboard.values()];
	}

	// Layout State
	viewMode = $state<'grid' | 'list'>('grid');
	sidebarOpen = $state(!this.#isMobile.current);
	previewOpen = $state(false);

	#api: FileManagerAPI;

	async deleteFile(ids: (TreeFile | TreeDirectory)[]): Promise<boolean> {
		for (const id of ids) {
			if ('directories' in id && (id.directories.length || id.files)) {
				if (confirm('Are you sure you want to delete the directory? It is not empty')) {
					break;
				} else {
					return false;
				}
			}
		}
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

	async createFolder(folderName: string): Promise<string> {
		if (folderName.includes('/')) {
			return "Folder name can not include slashes ('/')";
		}
		try {
			await this.#api.createFolder(
				(this.currentPath.type == CurrentPathType.Path ? this.currentPath.path : '/') +
					folderName +
					'/'
			);
			await this.refresh();
		} catch (error: any) {
			return error.body?.message || 'Folder creation failed';
		}
		return '';
	}

	async renameItem(id: TreeFile | TreeDirectory, newName: string): Promise<string> {
		try {
			// TODO: use id.id split magic, and for the love of god extract it into a utility function
			await this.#api.renameItem(id.id, `${id.id.split('/').slice(0, -1).join('/')}/${newName}`);
			await this.refresh();
		} catch (error: any) {
			return error.body?.message || 'Folder creation failed';
		}
		return '';
	}

	async moveItems(ids: (TreeFile | TreeDirectory)[], newDirectory: string): Promise<string> {
		if (!ids.length) {
			return '';
		}
		try {
			await this.#api.moveItems(
				ids.map((i) => i.id),
				newDirectory
			);
			await this.refresh();
		} catch (error: any) {
			return error.body?.message || 'Folder creation failed';
		}
		this.#clipboard.clear();
		return '';
	}

	#filterFileTypeObject = $state<FileTypeFilterObject>(fileTypeFilters[0]);
	get filterFileTypeObject() {
		return this.#filterFileTypeObject;
	}
	get filterFileType() {
		return this.#filterFileTypeObject.value;
	}
	set filterFileType(filter: ValidFileTypeFilters) {
		this.#filterFileTypeObject =
			fileTypeFilters.find((f) => f.value === filter) ?? fileTypeFilters[0];
	}

	/** Check if a file matches the current file type filter */
	matchesFilter(file: TreeFile): boolean {
		if (!this.#filterFileTypeObject.type) return true;
		return fileExtensionType(file.name.split('.').at(-1)) === this.#filterFileTypeObject.type;
	}

	constructor(
		api: FileManagerAPI,
		initialRoot: TreeDirectory,
		initialFilterFileType: ValidFileTypeFilters = 'all'
	) {
		this.#api = api;
		this.#root = $state<TreeDirectory>(initialRoot);
		this.filterFileType = initialFilterFileType;
	}

	/** Get the files matching the current file type filter in the currently active Directory */
	get currentFiles() {
		return this.filesInDirectory(this.currentDirectory);
	}

	/** Get the files matching the current file type filter in a given directory */
	filesInDirectory(directory: TreeDirectory) {
		return (directory.files ?? []).filter((f) => this.matchesFilter(f));
	}

	/** Get the direct subdirectories of the currently active Directory */
	get currentSubDirectories() {
		return this.currentDirectory?.directories ?? [];
	}

	/** If the current Directory is empty */
	currentEmpty() {
		return this.currentFiles.length == 0 && this.currentSubDirectories.length == 0;
	}

	/** Change currently active directory, either by string path, or by a `TreeDirectory` object */
	navigate(directory: TreeDirectory | string) {
		const targetPath = typeof directory === 'string' ? directory : directory.id;

		if (typeof directory === 'string' && !this.#traverseTree(targetPath)) {
			console.error(`${directory} was not found`);
			return;
		}

		this.clearSelection();
		this.#setParams({
			type: CurrentPathType.Path,
			path: targetPath,
			search: ''
		});
	}

	navigateRecent() {
		this.clearSelection();
		this.#setParams({
			type: CurrentPathType.Recent,
			path: '/',
			search: ''
		});
	}

	navigateSearch(search: string) {
		const isAlreadySearching = this.currentPath.type === CurrentPathType.Search;

		this.clearSelection();
		this.#setParams(
			{
				type: CurrentPathType.Search,
				path: '/',
				search
			},
			isAlreadySearching
		);
	}

	/** Get current URLSearchParam search value. Returns `''` if the `currentPath` isn't of type `Search` */
	get searchQuery() {
		return this.#params.search;
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
export function createFileManager(
	api: FileManagerAPI,
	root: TreeDirectory,
	initialFilterFileType: ValidFileTypeFilters = 'all'
): FileManager {
	return setContext(FM_KEY, new FileManager(api, root, initialFilterFileType));
}

/**
 * Retrieves the `FileManager` instance from the context. This is a class instance,
 * so you cannot destructure it.
 * @returns The `FileManager` instance.
 */
export function useFileManager() {
	return getContext<FileManager>(FM_KEY);
}
