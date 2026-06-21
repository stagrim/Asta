import type { TreeDirectory } from '$lib/api_bindings/files/TreeDirectory';
import type { RemoteQuery } from '@sveltejs/kit';
import type { File } from 'buffer';
import * as v from 'valibot';

export interface FileManagerAPI {
	getFileTree: () => RemoteQuery<TreeDirectory>;
	createFile: (file: File) => Promise<boolean>;
	createFolder: (id: string) => Promise<boolean>;
	deleteFile: (ids: string[]) => Promise<boolean>;
	renameItem: (id: string, newName: string) => Promise<boolean>;
	moveItems: (ids: string[], newName: string) => Promise<boolean>;
}

export enum CurrentPathType {
	Path = 'Path',
	Search = 'Search',
	Recent = 'Recent'
}

export const FileManagerSearchParams = v.object({
	type: v.optional(v.enum(CurrentPathType), CurrentPathType.Path),
	path: v.optional(v.string(), '/'),
	search: v.optional(v.string(), '')
});
