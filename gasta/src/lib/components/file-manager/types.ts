import type { TreeDirectory } from '#lib/server/sasta_client/index.ts';
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
	type: v.fallback(v.optional(v.enum(CurrentPathType), CurrentPathType.Path), CurrentPathType.Path),
	path: v.fallback(v.optional(v.string(), '/'), '/'),
	search: v.fallback(v.optional(v.string(), ''), '')
});

export enum FileTypes {
	PDF = 'pdf',
	Document = 'document',
	Spreadsheet = 'spreadsheet',
	Code = 'code',
	Image = 'image',
	Video = 'video',
	Audio = 'audio',
	Archive = 'archive',
	Unknown = 'unknown'
}

export type ValidFileTypeFilters = 'all' | 'image' | 'pdf';

export type FileTypeFilterObject = { value: ValidFileTypeFilters; label: string; type?: FileTypes };
export const fileTypeFilters: FileTypeFilterObject[] = [
	{ value: 'all', label: 'All files' },
	{ value: 'image', label: 'Images', type: FileTypes.Image },
	{ value: 'pdf', label: 'PDFs', type: FileTypes.PDF }
];
