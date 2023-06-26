<script lang="ts">
    import { page } from '$app/stores'
	import { RadioGroup, RadioItem, toastStore, type ToastSettings, modalStore } from '@skeletonlabs/skeleton';
	import type { PageData } from './$types'
	import SchedulePicker from '../../../lib/SchedulePicker.svelte';
	import { applyAction, enhance } from '$app/forms';
	import { invalidateAll } from '$app/navigation';
	import { form_action } from '$lib/form_action';

    export let data: PageData

    $: uuid = $page.params.uuid
    let display_schedule
    $: display_schedule = data.display.content.get(uuid)?.schedule
    let display_name
    $: display_name = data.display.content.get(uuid)?.name

    let chosen_schedule: Uuid
    const get_schedule_name = (uuid) => data.display.content.get(uuid).schedule
    // $: chosen_schedule = display_schedule
    $: chosen_schedule = get_schedule_name(uuid)

    let name_value: string
    $: name_value = display_name

    let delete_button: HTMLButtonElement

</script>

<form class="card m-4" method="POST" use:enhance={() => form_action}>
    <section class="p-4">
        <label class="label mb-5">
            <span>Name</span>
            <input required name="name" class="input" type="text" placeholder="Name must be unique" bind:value={name_value} />
        </label>

        <SchedulePicker name="schedule" bind:chosen_schedule schedules={data.schedule.content} />

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
