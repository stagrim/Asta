<script lang="ts">
	import cronstrue from 'cronstrue';
	import cron from 'cron-validate';
	import * as InputGroup from '$lib/components/ui/input-group';
	import { Clock } from '@lucide/svelte';
	import { Label } from '$lib/components/ui/label';
	import * as Dialog from '$lib/components/ui/dialog';
	import { Button, buttonVariants } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';

	let {
		value = $bindable(''),
		title
	}: {
		value: string;
		title?: string;
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

	const cron_readable = $derived.by(() => {
		try {
			return cronstrue.toString(value, {
				use24HourTimeFormat: true
			});
		} catch (e) {
			return '';
		}
	});
	let datetime = $state('');
	let open = $state(false);
</script>

{#snippet Test()}
	<Dialog.Root bind:open>
		<Dialog.Trigger type="button">
			<Clock />
		</Dialog.Trigger>
		<Dialog.Content>
			<Dialog.Header>
				<Dialog.Title>Date time please!</Dialog.Title>
				<Dialog.Description>
					Enter a date and time to be translated to a cron expression
				</Dialog.Description>
			</Dialog.Header>
			<Input class="w-full" type="datetime-local" bind:value={datetime} />
			<Dialog.Footer>
				<Dialog.Close class={buttonVariants({ variant: 'outline' })}>Cancel</Dialog.Close>
				<Button
					class={buttonVariants({ variant: 'default' })}
					onclick={() => {
						const d = new Date(datetime);
						value = `${d.getSeconds()} ${d.getMinutes()} ${d.getHours()} ${d.getDate()} ${d.getMonth() + 1} * ${d.getFullYear()}`;
						open = false;
					}}
				>
					Submit
				</Button>
			</Dialog.Footer>
		</Dialog.Content>
	</Dialog.Root>
{/snippet}

<div class="grid gap-2">
	{#if title}
		<Label>{title}</Label>
	{/if}

	<div class="relative w-full group">
		<InputGroup.Root>
			<InputGroup.Input
				class="peer"
				aria-invalid={!valid_cron}
				placeholder="cron format"
				bind:value
			/>
			<div
				class="absolute bottom-full mb-2 left-0 z-50 w-fit text-balance rounded-md px-3 py-1.5 bg-foreground text-background shadow-md invisible opacity-0 transition-all peer-focus:visible peer-focus:opacity-100"
			>
				<p class="text-xs">
					{#if valid_cron}
						{cron_readable}
					{:else}
						Cron expression is not valid
					{/if}
				</p>
			</div>
			<InputGroup.Addon align="inline-end">
				<InputGroup.Button variant="secondary">
					{@render Test()}
				</InputGroup.Button>
			</InputGroup.Addon>
		</InputGroup.Root>
	</div>
</div>
