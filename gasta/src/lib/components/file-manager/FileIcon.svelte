<script lang="ts">
	import FileTextIcon from '@lucide/svelte/icons/file-text';
	import FileSpreadsheetIcon from '@lucide/svelte/icons/file-spreadsheet';
	import FileCodeIcon from '@lucide/svelte/icons/file-code';
	import ImageIcon from '@lucide/svelte/icons/image';
	import VideoIcon from '@lucide/svelte/icons/video';
	import MusicIcon from '@lucide/svelte/icons/music';
	import ArchiveIcon from '@lucide/svelte/icons/archive';
	import FileIcon from '@lucide/svelte/icons/file';

	let {
		extension,
		size = 'md'
	}: {
		extension?: string;
		size?: 'sm' | 'md' | 'lg';
	} = $props();

	const sizeClasses = {
		sm: 'w-5 h-5',
		md: 'w-8 h-8',
		lg: 'w-12 h-12'
	};

	const extLookup: Record<string, string> = {
		pdf: 'pdf',
		png: 'image',
		jpg: 'image',
		jpeg: 'image',
		webp: 'image',
		txt: 'document'
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
		pdf: FileTextIcon,
		document: FileTextIcon,
		spreadsheet: FileSpreadsheetIcon,
		code: FileCodeIcon,
		image: ImageIcon,
		video: VideoIcon,
		audio: MusicIcon,
		archive: ArchiveIcon
	};

	const IconComponent = $derived(icons[type] || FileIcon);
	const colorClass = $derived(iconColors[type] || 'text-gray-400');
	const sizeClass = $derived(sizeClasses[size]);
</script>

<IconComponent class="{sizeClass} {colorClass}" />
