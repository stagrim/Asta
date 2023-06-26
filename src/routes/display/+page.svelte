<script lang="ts">
	import { applyAction, enhance } from "$app/forms";
	import { invalidateAll } from "$app/navigation";
	import { form_action } from "$lib/form_action";
	import SchedulePicker from "../../lib/SchedulePicker.svelte";
	import type { ActionData, PageData } from "./$types";
	import { toastStore } from '@skeletonlabs/skeleton';

    export let data: PageData
    
    let chosen_schedule: Uuid = "0"
</script>

<form class="card m-4" method="POST" action="?/create" use:enhance={() => { return form_action }}>
    <section class="p-4">

        <label class="label mb-5">
            <span>Name</span>
            <input required name="name" class="input" type="text" placeholder="Name must be unique" />
        </label>

        <SchedulePicker name="schedule" schedules={data.schedule.content} bind:chosen_schedule />

        <div class="flex w-full justify-center mt-5">
            <!-- Disable button when nothing is changed -->
            <button type="submit" class="btn variant-filled-primary">Add</button>
        </div>
    </section>
</form>
