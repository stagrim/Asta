import type { TreeDirectory } from '$lib/api_bindings/files/TreeDirectory';
import type { File } from 'buffer';

export interface FileManagerAPI {
	getFileTree: () => Promise<TreeDirectory>;
	createFile: (file: File) => Promise<boolean>;
	deleteFile: (ids: string[]) => Promise<boolean>;
	renameFile: (id: string, newName: string) => Promise<boolean>;
}
