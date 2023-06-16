<script lang="ts">
	import { applyAction, enhance } from "$app/forms";
	import { invalidateAll } from "$app/navigation";
	import type { Uuid } from "../../app";
	import SchedulePicker from "../../lib/SchedulePicker.svelte";
	import type { ActionData, PageData } from "./$types";
	import { toastStore } from '@skeletonlabs/skeleton';

    export let data: PageData
    
    let chosen_schedule: Uuid = "0"
</script>

<form class="card my-4" method="POST" action="?/create" use:enhance={({}) => {
    return async ({ result, update }) => {
        console.log(result)
            if (result.type === "success" || result.type === "redirect") {
                toastStore.trigger({
                    message: 'Display added',
                    background: 'variant-filled-success',
                    timeout: 2000
                })

                await invalidateAll()
                await applyAction(result)

            } else if (result.type === "failure") {
                toastStore.trigger({
                    message: result.data.message,
                    background: 'variant-filled-error',
                    autohide: false
                })
            }
    }
}}>
    <section class="p-4">

        <label class="label mb-5">
            <span>Name</span>
            <input required name="name" class="input" type="text" placeholder="Name must be unique" />
        </label>

        <SchedulePicker name="schedule" schedules={[...data.schedule.content.values()]} bind:chosen_schedule />

        <div class="flex w-full justify-center mt-5">
            <!-- Disable button when nothing is changed -->
            <button type="submit" class="btn variant-filled-primary">Add</button>
        </div>
    </section>
</form>
