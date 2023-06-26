<script lang="ts">
    import { page } from '$app/stores'
	import { RadioGroup, RadioItem, toastStore, type ToastSettings, modalStore } from '@skeletonlabs/skeleton';
	import type { PageData } from './$types'
	import SchedulePicker from '../../../lib/SchedulePicker.svelte';
	import { applyAction, enhance } from '$app/forms';
	import { invalidateAll } from '$app/navigation';
	import { form_action } from '$lib/form_action';
	import type { PlaylistItem } from '../../../api_bindings/update/PlaylistItem';
	import type { Playlist } from '../../../api_bindings/read/Playlist';
	import { append } from 'svelte/internal';

    export let data: PageData;

    $: uuid = $page.params.uuid
    $: playlist = data.playlist.content.get(uuid)

    let delete_button: HTMLButtonElement

    let playlist_items: PlaylistItem[]
    const get_items_copy = _ => structuredClone(playlist.items)
    $: playlist_items = get_items_copy(uuid)
        
    $: playlist_items, console.log(JSON.stringify(playlist_items ?? []))

    let add_value: "WEBSITE" | "IMAGE" | "TEXT"
    const add_item = () => playlist_items = [...playlist_items, { type: add_value, settings: {} }]

    const swap_item = (a: number, b: number) => {
        const tmp = playlist_items[a]
        playlist_items[a] = playlist_items[b]
        playlist_items[b] = tmp
    }
</script>

<form class="card m-4" method="POST" use:enhance={({ formData }) => {
    formData.append("items", JSON.stringify(playlist_items))
    return form_action
}}>
    <section class="p-4">

        {#if playlist}
            <label class="label mb-5">
                <span>Name</span>
                <input required name="name" class="input" type="text" placeholder="Name must be unique" bind:value={playlist.name} />
            </label>

            <h3 class="h3 mb-3">Items</h3>
            
            {#each playlist_items as item, i}
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
                                    title: `Delete '${item.name}'?`,
                                    body: `Are your sure you want to delete Item '${item.name}'?`,
                                    response: (r) => { if (r) { playlist_items.splice(i, 1); playlist_items = playlist_items } },
                                })
                            }
                            >&#128465;</button>
                            {#if i < playlist_items.length - 1 }
                                <button type="button" class="btn-icon btn-icon-sm variant-outline-primary" 
                                on:click={() => swap_item(i, i + 1)}
                                >&#8595;</button>
                            {/if}
                        </div>
                    </header>

                    <section class="p-4">
                        <label class="label mb-5">
                            <span>Name</span>
                            <input required class="input" type="text" placeholder="Name must be unique" bind:value={item.name} />
                        </label>
                        
                        {#if item.type == "WEBSITE"}
                            <label class="label mb-5">
                                <span>URL</span>
                                <input required class="input" type="text" placeholder="https://example.com" bind:value={item.settings.url} />
                            </label>
                        {:else if item.type == "TEXT"}
                            <label class="label mb-5">
                                <span>Text</span>
                                <input required class="input" type="text" placeholder="Some text..." bind:value={item.settings.text} />
                            </label>
                        {:else if item.type == "IMAGE"}
                            <label class="label mb-5">
                                <span>Image source</span>
                                <input required class="input" type="text" placeholder="https://example.com/src.png" bind:value={item.settings.src} />
                            </label>
                        {/if}

                        <label class="label mb-5">
                            <span>Durations</span>
                            <input required class="input" type="number" placeholder="Duration in seconds" bind:value={item.settings.duration} />
                        </label>
                    </section>
                
                </div>
            {/each}

            <label class="label mb-2 flex justify-center items-center">
                <span class="mr-2">Add</span>
                <select class="select" bind:value={add_value}>
                    <option value="WEBSITE">Website</option>
                    <option value="TEXT">Text</option>
                    <option value="IMAGE">Image</option>
                </select>
                <button type="button" class="btn-icon variant-filled-primary ml-2" on:click={add_item}>+</button>
            </label>

            <div class="flex w-full justify-center gap-4 mt-5">
                <button type="button" class="btn variant-ringed-error" on:click={() =>
                    modalStore.trigger({
                        type: 'confirm',
                        title: `Delete '${playlist.name}'?`,
                        body: `Are your sure you want to delete Display '${playlist.name}'?`,
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
