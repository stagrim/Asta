import { setContext, getContext } from 'svelte';
import type { TreeDirectory } from '$lib/api_bindings/files/TreeDirectory';
import type { TreeFile } from '$lib/api_bindings/files/TreeFile';
import { IsMobile } from '$lib/hooks/is-mobile.svelte';

const FM_KEY = Symbol('FILE_MANAGER');

export class FileManager {
	#root;
	/** Get root tree node */
	get root() {
		return this.#root;
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
	selectedItem = $state<TreeFile | TreeDirectory | null>(null);

	// Layout State
	viewMode = $state<'grid' | 'list'>('grid');
	sidebarOpen = $state(!this.#isMobile.current);
	previewOpen = $state(false);

	constructor(initialRoot: TreeDirectory) {
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
				this.selectedItem = null;
				this.#currentDirectory = dir;
			} else {
				console.error(`${directory} was not found`);
			}
		} else {
			this.#currentPath = directory.id;
			this.#currentDirectory = directory;
			this.selectedItem = null;
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
 * @param root The root TreeDirectory node of the file system
 * @returns  The `FileManager` instance.
 */
export function createFileManager(root: TreeDirectory): FileManager {
	return setContext(FM_KEY, new FileManager(root));
}

/**
 * Retrieves the `FileManager` instance from the context. This is a class instance,
 * so you cannot destructure it.
 * @returns The `FileManager` instance.
 */
export function useFileManager() {
	return getContext<FileManager>(FM_KEY);
}
