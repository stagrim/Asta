<script lang="ts">
	import { faClock } from '@fortawesome/free-solid-svg-icons/faClock';
	import { Icon } from 'svelte-awesome';

	import cronstrue from 'cronstrue';
	import cron from 'cron-validate';

	export let value: string;
	export let title: string;
	/**
	 * Index to give set_valid
	 */
	export let index: number;
	/**
	 * Runs when cron valid is evaluated
	 */
	export let set_valid: (number, boolean) => void;

	$: valid_cron = cron(value?.trim() ?? '', {
		override: {
			useSeconds: true,
			useYears: true,
			useAliases: true
		}
	}).isValid();
	$: set_valid(index, valid_cron);

	$: cron_readable =
		(() => {
			try {
				return cronstrue.toString(value, {
					use24HourTimeFormat: true
				});
			} catch (e) {}
		})() ?? '';

	const modal: ModalSettings = {
		type: 'prompt',
		title: 'Date time please!',
		body: 'Enter a date and time to be translated to a cron expression',
		valueAttr: { type: 'datetime-local', required: true },
		response: (r: string) => {
			if (r) {
				const d = new Date(r);
				value = `${d.getSeconds()} ${d.getMinutes()} ${d.getHours()} ${d.getDate()} ${d.getMonth() + 1} * ${d.getFullYear()}`;
			}
		}
	};
</script>

<div class="w-full">
	<label class="label mb-5">
		<span>{title}</span>
		<div class="input-group input-group-divider grid-cols-[1fr_auto]">
			<input
				required
				class="input pr-0"
				class:input-error={!valid_cron}
				type="text"
				placeholder="ss mm HH dd mm weekday YYYY"
				bind:value
			/>
			<button class="input-group-divider" type="button" on:click={() => modalStore.trigger(modal)}
				><Icon data={faClock}></Icon></button
			>
		</div>
	</label>
	{#if valid_cron}
		<span class="variant-soft-primary rounded-container-token p-[0.075rem]">{cron_readable}</span>
	{/if}

	<br />
	<br />
</div>
