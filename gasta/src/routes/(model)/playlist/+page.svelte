<script lang="ts">
	import { enhance } from '$app/forms';
	import { form_action } from '$lib/form_action';
</script>

<form class="card m-4" method="POST" action="?/create" use:enhance={() => form_action}>
	<section class="p-4">
		<label class="label mb-5">
			<span>Name</span>
			<input required name="name" class="input" type="text" placeholder="Name must be unique" />
		</label>

		<div class="flex w-full justify-center mt-5">
			<!-- Disable button when nothing is changed -->
			<button type="submit" class="btn variant-filled-primary">Add</button>
		</div>
	</section>
</form>
