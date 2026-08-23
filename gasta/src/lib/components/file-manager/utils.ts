import { FileTypes } from './types';

const extLookup: Record<string, FileTypes> = {
	pdf: FileTypes.PDF,
	png: FileTypes.Image,
	jpg: FileTypes.Image,
	jpeg: FileTypes.Image,
	webp: FileTypes.Image,
	svg: FileTypes.Image,
	gif: FileTypes.Image,
	txt: FileTypes.Document,
	zip: FileTypes.Archive,
	tar: FileTypes.Archive
};

export function fileExtensionType(extension?: string): FileTypes {
	if (!extension || !(extension in extLookup)) {
		return FileTypes.Unknown;
	}
	return extLookup[extension];
}
