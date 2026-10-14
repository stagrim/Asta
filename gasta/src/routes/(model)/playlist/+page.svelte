<script lang="ts">
	import { Button } from '#lib/components/ui/button/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import { Label } from '#lib/components/ui/label/index.js';
	import { createPlaylist } from './playlists.remote';
	import { toast } from 'svelte-sonner';

	const { name } = createPlaylist.fields;
</script>

<form
	class="w-full max-w-7xl"
	{...createPlaylist.enhance(async ({ submit }) => {
		try {
			if (await submit()) {
				toast.success('Playlist Added');
			} else {
				toast.error(createPlaylist.fields.allIssues()?.[0]?.message || 'Failed to add playlist');
			}
		} catch (error: any) {
			toast.error(error.body?.message || 'Failed to add playlist');
		}
	})}
>
	<div class="grid gap-2 mb-5">
		<Label>Name</Label>
		<Input {...name.as('text')} required class="input" placeholder="Name must be unique" />
	</div>

	<div class="mt-8">
		<Button type="submit">Add</Button>
	</div>
</form>
