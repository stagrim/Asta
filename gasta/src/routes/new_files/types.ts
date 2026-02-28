import type { TreeDirectory } from '$lib/api_bindings/files/TreeDirectory';
import type { File } from 'buffer';

export interface FileManagerAPI {
	getFileTree: () => Promise<TreeDirectory>;
	createFile: (file: File) => Promise<boolean>;
	deleteFile: (uuid: string) => Promise<boolean>;
	renameFile: (uuid: string, newName: string) => Promise<boolean>;
}
