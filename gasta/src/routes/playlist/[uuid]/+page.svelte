<script lang="ts">
    import { Icon } from 'svelte-awesome';
	import plus from 'svelte-awesome/icons/plus';
	import arrowDown from 'svelte-awesome/icons/arrowDown';
	import arrowUp from 'svelte-awesome/icons/arrowUp';
	import trash from 'svelte-awesome/icons/trash';
    
    import { generate } from "random-words";
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

    export let data: PageData;

    $: uuid = $page.params.uuid
    const get_playlist = (uuid) => structuredClone(data.playlist.content.get(uuid))
    $: playlist = get_playlist(uuid)

    let delete_button: HTMLButtonElement
        
    // $: playlist.items, console.log(JSON.stringify(playlist?.items ?? []))

    let add_value: "WEBSITE" | "IMAGE" | "TEXT"
    const add_item = () => 
        playlist.items = [...playlist.items, { 
            type: add_value,
            name: generate({ exactly: 1, wordsPerString: 2, separator: "-" })[0],
            settings: { duration: 60 } 
        }]

    const swap_item = (a: number, b: number) => {
        const tmp = playlist.items[a]
        playlist.items[a] = playlist.items[b]
        playlist.items[b] = tmp
    }
</script>

<form class="card m-4" method="POST" use:enhance={({ formData }) => {
    formData.append("items", JSON.stringify(playlist.items))
    return form_action
}}>
    <section class="p-4">

        {#if playlist}
            <label class="label mb-5">
                <span>Name</span>
                <input required name="name" class="input" type="text" placeholder="Name must be unique" bind:value={playlist.name} />
            </label>

            <div class="flex items-center justify-between w-full my-5">
                <h3 class="h3">Items</h3>

                <div class="flex justify-end items-center w-1/2">

                    <!-- <label class="label mb-2 flex justify-center items-center"> -->
                        <!-- <span class="mr-2">Add</span> -->
                    <select class="select w-3/4" bind:value={add_value}>
                        <option value="WEBSITE">Website</option>
                        <option value="TEXT">Text</option>
                        <option value="IMAGE">Image</option>
                    </select>
                    <button type="button" class="btn-icon btn-icon-sm variant-soft-primary ml-2" on:click={add_item}>
                        <Icon data={plus} scale=0.75 />
                    </button>
                </div>
                <!-- </label> -->
            </div>
            
            {#if playlist.items}
            {#each playlist.items as item, i}
                <div class="card mb-4">
                    <header class="card-header">
                        <div class="flex w-full justify-center gap-4">
                            {#if i > 0}
                                <button type="button" class="btn-icon btn-icon-sm variant-outline-primary"
                                on:click={() => swap_item(i, i - 1)}>
                                    <Icon data={arrowUp} scale=0.75 />
                                </button>
                            {/if}
                            <button type="button" class="btn-icon btn-icon-sm variant-filled-error"
                            on:click={() => { playlist.items.splice(i, 1); playlist.items = playlist.items }}>
                                <Icon data={trash} scale=0.75 />
                            </button>
                            {#if i < playlist.items.length - 1 }
                                <button type="button" class="btn-icon btn-icon-sm variant-outline-primary" 
                                on:click={() => swap_item(i, i + 1)}>
                                    <Icon data={arrowDown} scale=0.75 />
                                </button>
                            {/if}
                        </div>
                    </header>

                    <section class="p-4">
                        <div class="flex items-center gap-3">
                            <label class="label mb-5">
                                <span>Name</span>
                                <input required class="input" type="text" placeholder="Name must be unique" bind:value={item.name} />
                            </label>
    
                            <label class="label mb-5">
                                <span>Durations</span>
                                <input required class="input" type="number" placeholder="Duration in seconds" bind:value={item.settings.duration} />
                            </label>
                        </div>
                        
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
                    </section>
                
                </div>
            {/each}
            {/if}

            <div class="flex w-full justify-center gap-4 mt-5">
                <button type="button" class="btn variant-ringed-error" on:click={() =>
                    modalStore.trigger({
                        type: 'confirm',
                        title: `Delete '${playlist.name}'?`,
                        body: `Are your sure you want to delete Playlist '${playlist.name}'?`,
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
