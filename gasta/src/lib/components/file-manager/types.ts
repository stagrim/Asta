import type { TreeDirectory } from '$lib/api_bindings/files/TreeDirectory';
import type { RemoteQuery } from '@sveltejs/kit';
import type { File } from 'buffer';

export interface FileManagerAPI {
	getFileTree: () => RemoteQuery<TreeDirectory>;
	createFile: (file: File) => Promise<boolean>;
	createFolder: (id: string) => Promise<boolean>;
	deleteFile: (ids: string[]) => Promise<boolean>;
	renameFile: (id: string, newName: string) => Promise<boolean>;
}
