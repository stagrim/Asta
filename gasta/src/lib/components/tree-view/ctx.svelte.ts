import { setContext, getContext } from 'svelte';

const TREE_KEY = Symbol('tree-view');

export class TreeState {
	selectedId = $state<string | undefined>(undefined);

	constructor(initialSelectedId?: string) {
		this.selectedId = initialSelectedId;
	}

	select(id: string) {
		this.selectedId = id;
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
