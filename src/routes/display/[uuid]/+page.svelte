<script lang="ts">
    import { page } from '$app/stores'
	import { RadioGroup, RadioItem, toastStore, type ToastSettings } from '@skeletonlabs/skeleton';
	import type { PageData } from './$types'
	import type { Uuid } from '../../../app';
	import SchedulePicker from '../../../lib/SchedulePicker.svelte';
	import { applyAction, enhance } from '$app/forms';
	import { invalidateAll } from '$app/navigation';

    export let data: PageData

    let uuid
    $: uuid = $page.params.uuid
    let display_schedule
    $: display_schedule = data.display.content.get(uuid)?.schedule
    let display_name
    $: display_name = data.display.content.get(uuid)?.name

    let chosen_schedule: Uuid
    // $: console.log(chosen_schedule)
    // $: if (display_schedule) {
    //     // console.log("uuid has changed")
    //     chosen_schedule = display_schedule
    // }
    $: chosen_schedule = display_schedule

    let name_value: string
    $: name_value = display_name

    let visible: boolean = false;

</script>

<!-- <RadioGroup rounded="rounded-container-token" display="flex-col"> -->
<form class="card" method="POST" use:enhance={({}) => {
    return async ({ result }) => {
        console.log(result)
        if (result.type === "success" || result.type === "redirect") {
            toastStore.trigger({
                message: `Display ${ result.type === "success" ? "updated" : "deleted"}`,
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
            <input required name="name" class="input" type="text" placeholder="Name must be unique" bind:value={name_value} />
        </label>

        <SchedulePicker name="schedule" schedules={[...data.schedule.content.values()]} bind:chosen_schedule />

        <div class="flex w-full justify-center gap-4 mt-5">
            <!-- Disable button when nothing is changed -->
            <button type="button" class="btn variant-ringed-error" on:click={() => visible = true}>Delete</button>
            <button class="btn variant-filled-primary" formaction="?/update">Apply</button>
        </div>

        <div class="fixed w-full h-full left-0 top-0 variant-glass-surface transition" style:display={visible ? 'block' : 'none'}>
            <aside class="alert w-2/3 translate-x-1/4 translate-y-10 variant-filled-surface">
                <div class="alert-message">
                    <h3 class="h3">Delete '{display_name}'?</h3>
                    <p>Are your sure you want to delete Display '{display_name}'</p>
                </div>
                <div class="alert-actions flex w-full justify-end gap-2 mt-5">
                    <button type="button" class="btn variant-ringed-error" on:click={() => visible = false}>Cancel</button>
                    <button class="btn variant-filled-error" on:click={() => visible = false} formaction="?/delete">Delete</button>
                </div>
            </aside>
        </div>
    </section>

</form>
<!-- </RadioGroup> -->
