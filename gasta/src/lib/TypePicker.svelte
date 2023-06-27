<script lang="ts">
	import type { Playlist } from "../api_bindings/read/Playlist";
	import type { Schedule } from "../api_bindings/read/Schedule";
	import type { State } from "../app";


    export let types: State

    /** Bind value to react to user changes */
    export let chosen_type: string
    $: types_values = [...types.content.values()].sort((a, b) => a.name.localeCompare(b.name))

    /** Name property of the select element for form */
    export let name = ""
</script>

<label class="label">
    <span>{types.type}</span>

    <div class="flex flex-row items-center">

        <select required {name} class="select" bind:value={chosen_type}>
            {#each types_values as type (type.uuid) }
                <option value={type.uuid}>{type.name}</option>
            {/each}
        </select>

    </div>
</label>
