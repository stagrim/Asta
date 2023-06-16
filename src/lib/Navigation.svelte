<script lang="ts">
	import { page } from "$app/stores";
	import { Accordion, AccordionItem, drawerStore } from "@skeletonlabs/skeleton";

    const drawerClose = () => drawerStore.close()

    export let data: LayoutData

    const create_kind = (kind: string) => ({ values: data[kind].values, type: kind })

    let kinds
    $: data, kinds = ['display', 'schedule', 'playlist'].map(create_kind)
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<nav class="list-nav">

    <Accordion>
        {#each kinds as kind}
            {@const capitalized = `${kind.type.charAt(0).toUpperCase()}${kind.type.substring(1)}`}
            <AccordionItem open={$page.url.pathname.startsWith(`/${kind.type}`)}>
                <svelte:fragment slot="lead"></svelte:fragment>
                <svelte:fragment slot="summary">
                    <h3 class="h3">
                        {capitalized}s
                    </h3>
                </svelte:fragment>
                <svelte:fragment slot="content">
                    <ul>
                        {#each kind.values as value}
                            {@const href = `/${kind.type}/${value.uuid}`}
                            <li>
                                <a {href} on:click={drawerClose} class={href === $page.url.pathname ? 'bg-gradient-to-br from-primary-500 to-secondary-500' : ''}>
                                    <span class="flex-auto p-1 whitespace-pre-wrap leading-5">{value.name}</span>
                                </a>
                            </li>
                        {/each}
                
                        <li>
                            <a href={`/${kind.type}/`} on:click={drawerClose} class={`/${kind.type}` === $page.url.pathname ? 'variant-glass-primary' : ''}>
                                <span class="badge-icon p-4 variant-soft-primary">+</span>
                                <span class="flex-auto py-1 whitespace-pre-wrap italic">Create {capitalized}</span>
                            </a>
                        </li>
                    </ul>
                </svelte:fragment>
            </AccordionItem>
        {/each}
    </Accordion>
</nav>
