<script lang="ts">
	import * as Select from '$lib/components/ui/select';
	import * as InputGroup from '$lib/components/ui/input-group';

	import type { State } from '../app';
	import { ArrowRight, SearchIcon } from '@lucide/svelte';
	import InputGroupSelectTrigger from './components/ui/InputGroupSelectTrigger.svelte';

	let {
		types,
		label = true,
		// BUG: Should always have a valid uuid
		chosen_type = $bindable(''),
		name = ''
	}: {
		types: State;
		label?: boolean;
		/** Bind value to react to user changes */

		chosen_type?: string;
		/** Name property of the select element for form */
		name?: string;
	} = $props();

	let types_values = $derived(
		[...types.content.values()].sort((a, b) => a.name.localeCompare(b.name))
	);
</script>

<label class="label">
	{#if label}
		<span>{types.type}</span>
	{/if}

	<InputGroup.Root>
		<Select.Root required={true} {name} bind:value={chosen_type} type="single">
			<InputGroupSelectTrigger class="w-full">
				{types_values.find((v) => v.uuid == chosen_type)?.name}
			</InputGroupSelectTrigger>
			<Select.Content>
				<Select.Label>{types.type}s</Select.Label>
				{#each types_values as type (type.uuid)}
					<Select.Item value={type.uuid}>{type.name}</Select.Item>
				{/each}
			</Select.Content>
		</Select.Root>
		<InputGroup.Addon align="inline-end">
			{#if chosen_type !== '0'}
				<a href={`/${types.type.toLocaleLowerCase()}/${chosen_type}`}>
					<InputGroup.Button variant="secondary"><ArrowRight /></InputGroup.Button>
				</a>
			{/if}
		</InputGroup.Addon>
	</InputGroup.Root>
</label>
