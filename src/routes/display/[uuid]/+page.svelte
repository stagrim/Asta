<script lang="ts">
    import { page } from '$app/stores'
	import { RadioGroup, RadioItem, toastStore, type ToastSettings, modalStore } from '@skeletonlabs/skeleton';
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
    $: chosen_schedule = display_schedule

    let name_value: string
    $: name_value = display_name

    let delete_button: HTMLButtonElement

</script>

<form class="card m-4" method="POST" use:enhance={({}) => {
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

        <SchedulePicker name="schedule" schedules={data.schedule.values} bind:chosen_schedule />

        <div class="flex w-full justify-center gap-4 mt-5">
            <button type="button" class="btn variant-ringed-error" on:click={() =>
                modalStore.trigger({
                    type: 'confirm',
                    title: `Delete '${display_name}'?`,
                    body: `Are your sure you want to delete Display '${display_name}'?`,
                    response: (r) => r ? delete_button.click() : '',
                })
            }>Delete</button>
            <!-- Disable button when nothing is changed -->
            <button class="btn variant-filled-primary" formaction="?/update">Apply</button>
        </div>

        <button class="hidden" formaction="?/delete" bind:this={delete_button}/>
    </section>

</form>
