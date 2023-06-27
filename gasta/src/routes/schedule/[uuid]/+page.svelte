<script lang="ts">
    import { page } from '$app/stores'
	import { RadioGroup, RadioItem, toastStore, type ToastSettings, modalStore } from '@skeletonlabs/skeleton';
	import type { PageData } from './$types'
	import SchedulePicker from '../../../lib/TypePicker.svelte';
	import { applyAction, enhance } from '$app/forms';
	import { invalidateAll } from '$app/navigation';
	import { form_action } from '$lib/form_action';
	import type { PlaylistItem } from '../../../api_bindings/update/PlaylistItem';
	import type { Playlist } from '../../../api_bindings/read/Playlist';
	import { append } from 'svelte/internal';
	import type { ScheduledPlaylistInput } from '../../../api_bindings/update/ScheduledPlaylistInput';
	import TypePicker from '../../../lib/TypePicker.svelte';

    export let data: PageData;

    $: uuid = $page.params.uuid
    $: schedule = data.schedule.content.get(uuid)

    let delete_button: HTMLButtonElement

    let scheduled_playlists: ScheduledPlaylistInput[]
    const get_scheduled_copy = _ => structuredClone(schedule.scheduled) ?? []
    $: scheduled_playlists = get_scheduled_copy(uuid)
        
    const add_item = () => scheduled_playlists = [...scheduled_playlists, {}]

    const swap_item = (a: number, b: number) => {
        const tmp = scheduled_playlists[a]
        scheduled_playlists[a] = scheduled_playlists[b]
        scheduled_playlists[b] = tmp
    }
</script>

<form class="card m-4" method="POST" use:enhance={({ formData }) => {
    formData.append("scheduled", JSON.stringify(scheduled_playlists))
    return form_action
}}>
    <section class="p-4">

        {#if schedule}
            <label class="label mb-5">
                <span>Name</span>
                <input required name="name" class="input" type="text" placeholder="Name must be unique" bind:value={schedule.name} />
            </label>

            <TypePicker types={data.playlist} name="playlist" bind:chosen_type={schedule.playlist} />

            <h3 class="h3 mb-3 mt-5">Scheduled</h3>
            
            {#each scheduled_playlists as scheduled_playlist, i}
                <div class="card mb-4">
                    <header class="card-header">
                        <div class="flex w-full justify-center gap-4">
                            {#if i > 0}
                                <button type="button" class="btn-icon btn-icon-sm variant-outline-primary"
                                on:click={() => swap_item(i, i - 1)}
                                >&#8593;</button>
                            {/if}
                            <button type="button" class="btn-icon btn-icon-sm variant-filled-error"
                            on:click={() =>
                                modalStore.trigger({
                                    type: 'confirm',
                                    title: `Delete Scheduled Item?`,
                                    body: `Are your sure you want to delete the Scheduled Item?`,
                                    response: (r) => { if (r) { scheduled_playlists.splice(i, 1); scheduled_playlists = scheduled_playlists } },
                                })
                            }
                            >&#128465;</button>
                            {#if i < scheduled_playlists.length - 1 }
                                <button type="button" class="btn-icon btn-icon-sm variant-outline-primary" 
                                on:click={() => swap_item(i, i + 1)}
                                >&#8595;</button>
                            {/if}
                        </div>
                    </header>

                    <section class="p-4">
                        <TypePicker types={data.playlist} bind:chosen_type={scheduled_playlist.playlist} />
                        
                        <label class="label mb-5">
                            <span>Start</span>
                            <input required class="input" type="text" placeholder="ss mm HH dd mm weekday YYYY" bind:value={scheduled_playlist.start} />
                        </label>

                        <label class="label mb-5">
                            <span>End</span>
                            <input required class="input" type="text" placeholder="" bind:value={scheduled_playlist.end} />
                        </label>
                    </section>
                
                </div>
            {/each}

            <div class="mb-2 flex justify-center">
                <button type="button" class="btn-icon variant-filled-primary ml-2" on:click={add_item}>+</button>
            </div>

            <div class="flex w-full justify-center gap-4 mt-5">
                <button type="button" class="btn variant-ringed-error" on:click={() =>
                    modalStore.trigger({
                        type: 'confirm',
                        title: `Delete '${schedule.name}'?`,
                        body: `Are your sure you want to delete Display '${schedule.name}'?`,
                        response: (r) => r ? delete_button.click() : '',
                    })
                }>Delete</button>
                <!-- Disable button when nothing is changed -->
                <button class="btn variant-filled-primary" formaction="?/update">Apply</button>
            </div>

            <button class="hidden" formaction="?/delete" bind:this={delete_button}/>
        {/if}
    </section>

</form>
