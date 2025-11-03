<script lang="ts">
	import { getModalStore } from '@skeletonlabs/skeleton';
	import type { PageData } from './$types';
	import SvelteMarkdown from 'svelte-markdown';
	import Header from '$lib/markdown_renderers/Header.svelte';
	import Link from '$lib/markdown_renderers/Link.svelte';
	import { SignOut } from '@auth/sveltekit/components';

	export let data: PageData;
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

	{#if data.name}
		<SignOut class="mx-auto text-center mt-12" provider="authentik">
			<span slot="submitButton" class="text-center btn variant-ghost-error mx-auto block relative"
				>Log out</span
			>
		</SignOut>
	{/if}

	{#if data.markdown}
		<div class="mt-10 p-4">
			<SvelteMarkdown source={data.markdown} renderers={{ heading: Header, link: Link }} />
		</div>
	{/if}
</div>
