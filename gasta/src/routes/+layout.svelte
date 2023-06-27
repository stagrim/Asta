<script lang="ts">
	// Your selected Skeleton theme:
	import '../theme.css';
	// This contains the bulk of Skeletons required styles:
	import '@skeletonlabs/skeleton/styles/skeleton.css';
	// Finally, your application's global stylesheet (sometimes labeled 'app.css')
	import '../app.postcss';

    import { AppBar, AppShell, Toast, Drawer, drawerStore, LightSwitch, Modal } from '@skeletonlabs/skeleton';

    import { page } from '$app/stores'
    import type { LayoutData } from './$types';
	import Navigation from '$lib/Navigation.svelte';

    export let data: LayoutData

	// TODO: Add check for authentication here and update store variable
</script>

<Toast />

<Modal />

<AppBar gridColumns="grid-cols-3" slotDefault="place-self-center" slotTrail="place-content-end">
	<svelte:fragment slot="lead">
        <button class="md:hidden btn btn-sm mr-4" on:click={() => drawerStore.open({ }) }>
            <span>
                <svg viewBox="0 0 100 80" class="fill-token w-4 h-4">
                    <rect width="100" height="20" />
                    <rect y="30" width="100" height="20" />
                    <rect y="60" width="100" height="20" />
                </svg>
            </span>
        </button>
    </svelte:fragment>
	    
        <h4 class="h4">Asta Admin</h4>

	<svelte:fragment slot="trail">
        <LightSwitch rounded="rounded-full" />
    </svelte:fragment>
</AppBar>

<Drawer>
    <div class="p-2">
        <Navigation {data} />
    </div>
</Drawer>

<AppShell slotSidebarLeft="max-w-xs w-0 md:w-2/6 md:p-4 h-screen overflow-y-scroll">
    <svelte:fragment slot="sidebarLeft">
        <Navigation {data} />
    </svelte:fragment>
    <slot />
</AppShell>

