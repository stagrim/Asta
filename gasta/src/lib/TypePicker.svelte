<script lang="ts">
	import * as Select from '#lib/components/ui/select/index.js';
	import * as InputGroup from '#lib/components/ui/input-group/index.js';

	import type { State } from '../app';
	import { ArrowRight } from '@lucide/svelte';
	import InputGroupSelectTrigger from './components/ui/InputGroupSelectTrigger.svelte';

	let {
		types,
		label = true,
		// BUG: Should always have a valid uuid
		chosen_type = $bindable(''),
		value = $bindable(),
		name = ''
	}: {
		types: State;
		label?: boolean;
		/** Bind value to react to user changes */
		chosen_type?: string;
		value?: string;
		/** Name property of the select element for form */
		name?: string;
	} = $props();

	let types_values = $derived(
		[...types.content.values()].sort((a, b) => a.name.localeCompare(b.name))
	);
	let selected = $derived(value ?? chosen_type);
</script>

<label class="label">
	{#if label}
		<span>{types.type}</span>
	{/if}

	<InputGroup.Root>
		<Select.Root
			required={true}
			{name}
			bind:value={
				() => selected,
				(v) => {
					value = v;
					chosen_type = v;
				}
			}
			type="single"
		>
			<InputGroupSelectTrigger class="w-full">
				{types_values.find((v) => v.uuid == selected)?.name}
			</InputGroupSelectTrigger>
			<Select.Content>
				<Select.Label>{types.type}s</Select.Label>
				{#each types_values as type (type.uuid)}
					<Select.Item value={type.uuid}>{type.name}</Select.Item>
				{/each}
			</Select.Content>
		</Select.Root>
		<InputGroup.Addon align="inline-end">
			{#if selected !== '0'}
				<a href={`/${types.type.toLocaleLowerCase()}/${selected}`}>
					<InputGroup.Button variant="secondary"><ArrowRight /></InputGroup.Button>
				</a>
			{/if}
		</InputGroup.Addon>
	</InputGroup.Root>
</label>
