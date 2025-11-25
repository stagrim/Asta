import type { PlaylistItem } from '$lib/api_bindings/update/PlaylistItem.ts';

export function getData(item: PlaylistItem) {
	switch (item.type) {
		case 'WEBSITE':
			return item.settings.url;
		case 'TEXT':
			return item.settings.text;
		case 'IMAGE':
			return item.settings.src;
		case 'BACKGROUND_AUDIO':
			return item.settings.src;
	}
}
