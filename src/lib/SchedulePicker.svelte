<script lang="ts">
	import type { Schedule, Uuid } from "../app";

    export let schedules: Schedule[]
    export let chosen_schedule: Uuid
    export let name = ""
</script>

<label class="label">
    <span>Schedule</span>

    <div class="flex flex-row items-center">

        <select required {name} class="select" bind:value={chosen_schedule}>
            {#each schedules as schedule (schedule.uuid) }
            <option value={schedule.uuid}>{schedule.name}</option>
            <!-- <RadioItem bind:group={radio_schedule} name="justify" value={schedule.uuid}>{schedule.name}</RadioItem> -->
            {/each}
        </select>
        <!-- {#if chosen_schedule !== display_schedule}
            <span
                class="badge ml-5 input-warning cursor-pointer"
                on:click={_ => chosen_schedule = display_schedule}
            >Reset</span>
        {/if} -->

    </div>
</label>
