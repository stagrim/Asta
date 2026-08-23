<script lang="ts">
	import VideoIcon from '@lucide/svelte/icons/video';
	import FileIcon from '@lucide/svelte/icons/file';
	import {
		FileArchive,
		FileCode,
		FileImage,
		FileMusic,
		FileSpreadsheet,
		FileText
	} from '@lucide/svelte';
	import { FileTypes } from './types';
	import { fileExtensionType } from './utils';

	let {
		extension,
		size = 'md'
	}: {
		extension?: string;
		size?: 'sm' | 'md' | 'lg' | 'xl';
	} = $props();

	const sizeClasses = {
		sm: 'w-5 h-5',
		md: 'w-8 h-8',
		lg: 'w-12 h-12',
		xl: 'w-24 h-24'
	};

	// let type = $derived(extension ? extLookup[extension] : 'archive');

	const iconColors: Record<string, string> = {
		pdf: 'text-red-500',
		document: 'text-blue-500',
		spreadsheet: 'text-green-500',
		code: 'text-yellow-500',
		image: 'text-purple-500',
		video: 'text-pink-500',
		audio: 'text-orange-500',
		archive: 'text-gray-500'
	};

	const icons: Record<FileTypes, [typeof FileIcon, string]> = {
		[FileTypes.PDF]: [FileText, 'text-red-500'],
		[FileTypes.Document]: [FileText, 'text-red-500'],
		[FileTypes.Spreadsheet]: [FileSpreadsheet, 'text-green-500'],
		[FileTypes.Code]: [FileCode, 'text-yellow-500'],
		[FileTypes.Image]: [FileImage, 'text-purple-500'],
		[FileTypes.Video]: [VideoIcon, 'text-pink-500'],
		[FileTypes.Audio]: [FileMusic, 'text-orange-500'],
		[FileTypes.Archive]: [FileArchive, 'text-gray-500'],
		[FileTypes.Unknown]: [FileArchive, 'text-gray-500']
	};

	const [IconComponent, colorClass] = $derived(icons[fileExtensionType(extension)]);
	const sizeClass = $derived(sizeClasses[size]);
</script>

<IconComponent class="{sizeClass} {colorClass}" />
