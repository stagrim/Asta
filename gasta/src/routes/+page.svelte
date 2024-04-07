<script lang="ts">
	import { getModalStore } from '@skeletonlabs/skeleton';
	import type { PageData } from './$types';
	import SvelteMarkdown from 'svelte-markdown';
	import Header from '$lib/markdown_renderers/Header.svelte';
	import Link from '$lib/markdown_renderers/Link.svelte';

	export let data: PageData;
	const modalStore = getModalStore();

	let logout_submit_button: HTMLButtonElement;
</script>

<div class="card m-8 max-w-4xl mx-auto p-8">
	<h1 class="h1 font-bold text-center">
		Welcome to
		<span
			class="bg-gradient-to-br from-primary-500 to-primary-300 bg-clip-text text-transparent box-decoration-clone"
		>
			Asta
		</span>
		Admin Panel
		<span
			class="bg-gradient-to-br from-primary-500 to-primary-300 bg-clip-text text-transparent box-decoration-clone"
		>
			{data.name}
		</span>
	</h1>

	{#if data.user}
		<form class="mt-12" method="POST" action="/login?/logout">
			<button
				class="btn variant-ghost-error mx-auto block relative"
				on:click={(e) => {
					e.preventDefault();
					modalStore.trigger({
						type: 'confirm',
						title: `Do you want to log out?`,
						response: (r) => (r ? logout_submit_button.click() : '')
					});
				}}>Log out</button
			>

			<button class="hidden" bind:this={logout_submit_button} />
		</form>
	{/if}

	{#if data.markdown}
		<div class="mt-10 p-4">
			<SvelteMarkdown source={data.markdown} renderers={{ heading: Header, link: Link }} />
		</div>
	{/if}
</div>
