import type { TreeFile } from '$lib/server/sasta_client';
import { setContext, getContext } from 'svelte';

const TREE_KEY = Symbol('tree-view');

export class TreeState {
	selectedId = $state<string | undefined>(undefined);
	complementryData = $state<TreeFile | undefined>(undefined);

	constructor(initialSelectedId?: string, initialComplementryData?: TreeFile) {
		this.selectedId = initialSelectedId;
		this.complementryData = initialComplementryData;
	}

	select(id: string, data?: TreeFile) {
		this.selectedId = id;
		this.complementryData = data;
	}
}

export function setTreeContext(initialSelectedId?: string) {
	const treeState = new TreeState(initialSelectedId);
	setContext(TREE_KEY, treeState);
	return treeState;
}

export function getTreeContext() {
	return getContext<TreeState>(TREE_KEY);
}
