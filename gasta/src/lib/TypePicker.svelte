<script lang="ts">
	import arrowRight from 'svelte-awesome/icons/arrowRight';
	import * as Select from '$lib/components/ui/select';
	import * as InputGroup from '$lib/components/ui/input-group';

	import { Icon } from 'svelte-awesome';
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

	<div class="flex flex-row items-center input-group input-group-divider cursor-pointer">
		<!-- <select
			required
			{name}
			class="select"
			style="min-width: unset !important;"
			bind:value={chosen_type}
		>
			{#each types_values as type (type.uuid)}
				<option value={type.uuid}>{type.name}</option>
			{/each}
		</select> -->

		<InputGroup.Root>
			<Select.Root required {name} bind:value={chosen_type} type="single">
				<InputGroupSelectTrigger class="w-full"
					>{types_values.find((v) => v.uuid == chosen_type)?.name}</InputGroupSelectTrigger
				>
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
				<!-- <a href={`/${types.type.toLocaleLowerCase()}/${chosen_type}`}>
					<div>
						<ArrowRight />
					</div>
				</a> -->
			</InputGroup.Addon>
		</InputGroup.Root>
		<!-- style="min-width: unset !important;" -->

		<!-- <a class="h-[40px] w-10" href={`/${types.type.toLocaleLowerCase()}/${chosen_type}`}>
			<div>
				<Icon data={arrowRight} scale={0.75} />
			</div>
		</a> -->
	</div>
</label>
