<script>
	export let depth;
	export let raw;
</script>

{#if depth === 1}
	<h1 class="h1 font-bold my-5"><slot /></h1>
{:else if depth === 2}
	<h2 class="h2 font-bold my-4"><slot /></h2>
{:else if depth === 3}
	<h3 class="h2 font-bold my-4"><slot /></h3>
{:else if depth === 4}
	<h4 class="h2 font-bold my-3"><slot /></h4>
{:else if depth === 5}
	<h5 class="h2 font-bold my-2"><slot /></h5>
{:else if depth === 6}
	<h6 class="h2 font-bold my-1"><slot /></h6>
{:else}
	{raw}
{/if}
