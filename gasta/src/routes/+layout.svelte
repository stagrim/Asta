<script lang="ts">
	// Finally, your application's global stylesheet (sometimes labeled 'app.css')
	import '../app.postcss';
    import { AppBar, AppShell, Toast, Drawer, getDrawerStore, LightSwitch, Modal, Avatar, initializeStores } from '@skeletonlabs/skeleton';
    import type { LayoutData } from './$types';
	import Navigation from '$lib/Navigation.svelte';

    export let data: LayoutData

    initializeStores()

    const drawerStore = getDrawerStore();

	// TODO: Add check for authentication here and update store variable
</script>

<Toast />

<Modal />

<Drawer>
    <div class="p-2">
        {#if !data.empty}
            <Navigation {data} />
        {/if}
    </div>
</Drawer>

<AppShell slotSidebarLeft={!data.empty ? "max-w-xs w-0 md:w-2/6 md:p-4 h-screen overflow-y-scroll" : ""} >
    <svelte:fragment slot="pageHeader">
		<AppBar gridColumns="grid-cols-3" slotDefault="place-self-center" slotTrail="place-content-end">
            <svelte:fragment slot="lead">
                {#if !data.empty}
                    <button class="md:hidden btn btn-sm mr-4" on:click={() => drawerStore.open({ }) }>
                        <span>
                            <svg viewBox="0 0 100 80" class="fill-token w-4 h-4">
                                <rect width="100" height="20" />
                                <rect y="30" width="100" height="20" />
                                <rect y="60" width="100" height="20" />
                            </svg>
                        </span>
                    </button>
                {/if}
            </svelte:fragment>

            <!-- <h4 class="h4">Asta Admin</h4> -->
            <a href={ data.user ? "/" : "" } style:cursor={ data.user ? "pointer" : "default" }>
                <Avatar rounded="rounded-full" src="/asta_icon.jpg" width="w-16" />
            </a>

            <svelte:fragment slot="trail">
                <LightSwitch rounded="rounded-full" />
            </svelte:fragment>
        </AppBar>
	</svelte:fragment>

    <svelte:fragment slot="sidebarLeft">
        {#if !data.empty}
            <Navigation {data} />
        {/if}
    </svelte:fragment>

    <slot />
</AppShell>

