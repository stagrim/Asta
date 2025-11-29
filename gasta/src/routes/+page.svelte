<script lang="ts">
	import type { PageData } from './$types';
	import SvelteMarkdown from 'svelte-markdown';
	import Header from '$lib/markdown_renderers/Header.svelte';
	import Link from '$lib/markdown_renderers/Link.svelte';
	import { SignOut } from '@auth/sveltekit/components';
	import Button from '$lib/components/ui/button/button.svelte';
	import * as Card from '$lib/components/ui/card/index.js';
	import { signOut } from '@auth/sveltekit/client';

	export let data: PageData;
</script>

<Card.Root class="max-w-sm w-full">
	<Card.Header class="text-center">
		<Card.Title class="text-xl">Welcome back {data.name}</Card.Title>
	</Card.Header>
	<Card.Content>
		<Button variant="destructive" onclick={() => signOut()}>Log out</Button>
	</Card.Content>
</Card.Root>
