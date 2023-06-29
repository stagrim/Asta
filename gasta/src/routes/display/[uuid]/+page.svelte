<script lang="ts">
    import { page } from '$app/stores'
	import { modalStore } from '@skeletonlabs/skeleton';
	import type { PageData } from './$types'
	import TypePicker from '../../../lib/TypePicker.svelte';
	import { enhance } from '$app/forms';
	import { form_action } from '$lib/form_action';

    export let data: PageData

    $: uuid = $page.params.uuid
    const get_display = (uuid) => structuredClone(data.display.content.get(uuid))
    $: display = get_display(uuid)

    let delete_button: HTMLButtonElement
</script>

<form class="card m-4" method="POST" use:enhance={() => form_action}>
    <section class="p-4">
        <label class="label mb-5">
            <span>Uuid</span>
            <input class="input" type="text" placeholder="Name must be unique" disabled value={display.uuid} />
        </label>

        <label class="label mb-5">
            <span>Name</span>
            <input required name="name" class="input" type="text" placeholder="Name must be unique" bind:value={display.name} />
        </label>

        <TypePicker name="schedule" bind:chosen_type={display.schedule} types={data.schedule} />

        <div class="flex w-full justify-center gap-4 mt-5">
            <button type="button" class="btn variant-ringed-error" on:click={() =>
                modalStore.trigger({
                    type: 'confirm',
                    title: `Delete '${display.name}'?`,
                    body: `Are your sure you want to delete Display '${display.name}'?`,
                    response: (r) => r ? delete_button.click() : '',
                })
            }>Delete</button>
            <!-- Disable button when nothing is changed -->
            <button class="btn variant-filled-primary" formaction="?/update">Apply</button>
        </div>

        <button class="hidden" formaction="?/delete" bind:this={delete_button}/>
    </section>

</form>
