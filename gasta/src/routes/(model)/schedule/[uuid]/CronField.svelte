<script lang="ts">
	import cronstrue from 'cronstrue';
	import cron from 'cron-validate';
	import { watch } from 'runed';
	import * as InputGroup from '$lib/components/ui/input-group';
	import { Clock } from '@lucide/svelte';
	import { Label } from '$lib/components/ui/label';

	let {
		value = $bindable(''),
		title,
		index,
		set_valid
	}: {
		value: string;
		title: string;
		/**
		 * Index to give set_valid
		 */ index: number;
		/**
		 * Runs when cron valid is evaluated
		 */ set_valid: (index: number, valid: boolean) => void;
	} = $props();

	const valid_cron: boolean = $derived(
		cron(value.trim(), {
			override: {
				useSeconds: true,
				useYears: true,
				useAliases: true
			}
		}).isValid()
	);

	watch(
		() => valid_cron,
		() => {
			set_valid(index, valid_cron);
		}
	);

	const cron_readable = $derived.by(() => {
		try {
			return cronstrue.toString(value, {
				use24HourTimeFormat: true
			});
		} catch (e) {
			return '';
		}
	});

	// const modal: ModalSettings = {
	// 	type: 'prompt',
	// 	title: 'Date time please!',
	// 	body: 'Enter a date and time to be translated to a cron expression',
	// 	valueAttr: { type: 'datetime-local', required: true },
	// 	response: (r: string) => {
	// 		if (r) {
	// 			const d = new Date(r);
	// 			value = `${d.getSeconds()} ${d.getMinutes()} ${d.getHours()} ${d.getDate()} ${d.getMonth() + 1} * ${d.getFullYear()}`;
	// 		}
	// 	}
	// };
</script>

<div class="grid gap-2 mb-5">
	<Label>{title}</Label>
	<InputGroup.Root>
		<InputGroup.Input placeholder="Type to search..." />
		<InputGroup.Addon align="inline-end">
			<InputGroup.Button variant="secondary"><Clock /></InputGroup.Button>
		</InputGroup.Addon>
	</InputGroup.Root>
	{#if valid_cron}
		<span class="variant-soft-primary rounded-container-token p-[0.075rem]">{cron_readable}</span>
	{/if}
</div>
