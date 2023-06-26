<script lang="ts">
	import type { Schedule } from "../api_bindings/read/Schedule";


    export let schedules: Map<string, Schedule>
    $: schedule_names = [...schedules.values()].sort((a, b) => a.name.localeCompare(b.name))
    export let chosen_schedule: string
    export let name = ""
</script>

<label class="label">
    <span>Schedule</span>

    <div class="flex flex-row items-center">

        <select required {name} class="select" bind:value={chosen_schedule}>
            {#each schedule_names as schedule (schedule.uuid) }
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
