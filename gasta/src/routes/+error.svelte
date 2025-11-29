<script>
	import { page } from '$app/state';
	import { Button } from '$lib/components/ui/button';
	import * as Card from '$lib/components/ui/card';
	import { signOut } from '@auth/sveltekit/client';

	const pathname = page.url.pathname;
	let errorTitle = $state('Page not Found');
	let errorDesc = $state('Sorry, it looks like the requested page could not be found.');
	let showReturn = $state(false);

	if (pathname.startsWith('/not-authorized')) {
		errorTitle = 'Access Denied';
		errorDesc = 'You do not have permission to sign in to Asta.';
		showReturn = true;
	}
</script>

<Card.Root class="max-w-lg self-center">
	<Card.Header>
		<Card.Title>{errorTitle}</Card.Title>
		<Card.Description>{errorDesc}</Card.Description>
	</Card.Header>
	<Card.Content>
		<div>
			<img src="/404.jpg" class="rounded-md" alt="" />
		</div>
	</Card.Content>
	<Card.Footer>
		{#if showReturn}
			<a href="/login">
				<Button variant="outline">Return</Button>
			</a>
		{/if}
	</Card.Footer>
</Card.Root>
