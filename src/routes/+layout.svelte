<script lang="ts">
	// Your selected Skeleton theme:
	import '../theme.css';
	// This contains the bulk of Skeletons required styles:
	import '@skeletonlabs/skeleton/styles/skeleton.css';
	// Finally, your application's global stylesheet (sometimes labeled 'app.css')
	import '../app.postcss';

    import { AppShell } from '@skeletonlabs/skeleton';

    import { page } from '$app/stores'
    import type { LayoutData } from './$types';

    export let data: LayoutData

	// TODO: Add check for authentication here and update store variable
</script>

<AppShell slotSidebarLeft="max-w-xs w-2/6 p-4 h-screen overflow-y-scroll">
    <svelte:fragment slot="sidebarLeft">
        <!-- Insert the list: -->
        <!-- <nav class="list-nav">
            <ul>
                <li><a href="/" class:!bg-primary-500={true} >Home</a></li>
                <li><a href="/about">About</a></li>
            </ul>
        </nav> -->
        <nav class="list-nav">
            <h2 class="h2 mb-5">Displays</h2>
            <ul>
                {#each data.display.content as display}
                    {@const href = `/display/${display.uuid}`}
                    <li>
                        <a {href} class={href === $page.url.pathname ? 'bg-gradient-to-br from-primary-500 to-secondary-500' : ''}>
                            <span class="flex-auto p-1">{display.name}</span>
                        </a>
                    </li>
                {/each}
            </ul>

            <h2 class="h2 my-5">Schedules</h2>
            <ul>
                {#each data.schedule.content as schedule}
                    {@const href = `/schedule/${schedule.uuid}`}
                    <li>
                        <a {href} class={href === $page.url.pathname ? 'bg-gradient-to-br from-primary-500 to-secondary-500' : ''}>
                            <span class="flex-auto p-1">{schedule.name}</span>
                        </a>
                    </li>
                {/each}
            </ul>

            <h2 class="h2 my-5">Playlist</h2>
            <ul>
                {#each data.playlist.content as playlist}
                    {@const href = `/playlist/${playlist.uuid}`}
                    <li>
                        <a {href} class={href === $page.url.pathname ? 'bg-gradient-to-br from-primary-500 to-secondary-500' : ''}>
                            <span class="flex-auto p-1 whitespace-pre-wrap leading-5">{playlist.name}</span>
                        </a>
                    </li>
                {/each}
            </ul>
        </nav>
        <!-- --- -->
    </svelte:fragment>
    <slot />
</AppShell>

