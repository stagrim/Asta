<script>
	import * as Card from '$lib/components/ui/card';
</script>

<Card.Root class="w-lg self-center">
	<Card.Header>
		<Card.Title>Browser not Supported</Card.Title>
		<Card.Description>
			Your browser is sadly not supported by the Asta system. Try updating to a newer version or
			switch to browsers known to be working because of strong security and trust, like Firefox,
			Chrome or Safari.
		</Card.Description>
	</Card.Header>
	<Card.Content>
		<div>
			<img src="/not_supported.png" class="rounded-md" alt="" />
		</div>
	</Card.Content>
</Card.Root>
