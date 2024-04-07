<script lang="ts">
	import arrowRight from 'svelte-awesome/icons/arrowRight';

	import { Icon } from 'svelte-awesome';
	import type { State } from '../app';

	export let types: State;

	/** Bind value to react to user changes */
	export let chosen_type: string = '';

	$: types_values = [...types.content.values()].sort((a, b) => a.name.localeCompare(b.name));

	/** Name property of the select element for form */
	export let name = '';
</script>

<label class="label">
	<span>{types.type}</span>

	<div
		class="flex flex-row items-center input-group input-group-divider grid-cols-[1fr_auto] cursor-pointer"
	>
		<select required {name} class="select" bind:value={chosen_type}>
			{#each types_values as type (type.uuid)}
				<option value={type.uuid}>{type.name}</option>
			{/each}
		</select>
		<!-- svelte-ignore a11y-missing-attribute -->
		<a class="h-[40px]" href={`/${types.type.toLocaleLowerCase()}/${chosen_type}`}>
			<div>
				<!-- <button type="button"> -->
				<Icon data={arrowRight} scale={0.75} />
				<!-- </button> -->
			</div>
		</a>
	</div>
</label>
