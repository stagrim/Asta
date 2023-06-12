<script lang="ts">
    import { page } from '$app/stores'
	import type { PageData } from './$types'

    export let data: PageData

    const uuid = $page.params.uuid
    const display = data.display.content[data.display.index.get(uuid)]
</script>

{display.name}
