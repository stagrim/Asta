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

	const extLookup: Record<string, string> = {
		pdf: 'pdf',
		png: 'image',
		jpg: 'image',
		jpeg: 'image',
		webp: 'image',
		svg: 'image',
		txt: 'document',
		zip: 'archive',
		tar: 'archive'
	};

	let type = $derived(extension ? extLookup[extension] : 'archive');

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

	const icons: Record<string, typeof FileIcon> = {
		pdf: FileText,
		document: FileText,
		spreadsheet: FileSpreadsheet,
		code: FileCode,
		image: FileImage,
		video: VideoIcon,
		audio: FileMusic,
		archive: FileArchive
	};

	const IconComponent = $derived(icons[type] || FileIcon);
	const colorClass = $derived(iconColors[type] || 'text-gray-400');
	const sizeClass = $derived(sizeClasses[size]);
</script>

<IconComponent class="{sizeClass} {colorClass}" />
