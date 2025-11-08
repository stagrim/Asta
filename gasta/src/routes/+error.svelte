<script>
	import { page } from '$app/stores';
	import { SignOut } from '@auth/sveltekit/components';

	const params = $page.url.searchParams;
	let errorTitle = $state('Page not Found');
	let errorDesc = $state('Sorry, it looks like the requested page could not be found.');
	let showReturn = $state(false);

	switch (params.get('error')) {
		case 'AccessDenied':
			errorTitle = 'Access Denied';
			errorDesc = 'You do not have permission to sign in to Asta.';
			showReturn = true;
			break;
		default:
			break;
	}
</script>

<div class="flex justify-center">
	<section class="p-4 card m-4 max-w-4xl">
		<h1 class="h1 py-4 font-semibold">{errorTitle}</h1>
		<div class="flex basis-0 gap-3 flex-col sm:flex-row">
			<div class="sm:w-1/2">
				<img src="/404.jpg" class="rounded-md" alt="" />
			</div>
			<div class="sm:w-1/2">{errorDesc}</div>
			{#if showReturn}
				<a href="/" class="block">
					<span class="text-center btn variant-ghost-primary block relative">Return</span>
				</a>
			{/if}
		</div>
	</section>
</div>
