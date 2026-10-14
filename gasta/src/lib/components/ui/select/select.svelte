<script lang="ts">
	import { Select as SelectPrimitive } from 'bits-ui';
	import { tick } from 'svelte';

	let {
		open = $bindable(false),
		value = $bindable(),
		...restProps
	}: SelectPrimitive.RootProps = $props();

	let marker: HTMLSpanElement | undefined = $state();
</script>

<SelectPrimitive.Root
	bind:open
	bind:value={
		() => value as never,
		(v) => {
			value = v;
			tick().then(() => {
				const input = marker?.previousElementSibling;
				if (input instanceof HTMLInputElement) {
					input.dispatchEvent(new Event('input', { bubbles: true }));
				}
			});
		}
	}
	{...restProps}
/>
{#if restProps.name}
	<span bind:this={marker} class="hidden" aria-hidden="true"></span>
{/if}
