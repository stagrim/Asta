<script lang="ts">
    import type { PageData } from './$types';

    export let data: PageData;
    // console.log(data.display)
</script>



<div class="card m-8 max-w-4xl mx-auto p-8">

    <h1 class="h1 font-bold text-center">
        Welcome to
        <span class="bg-gradient-to-br from-primary-500 to-primary-300 bg-clip-text text-transparent box-decoration-clone">
            Asta
        </span>
        Admin Panel
        <span class="bg-gradient-to-br from-primary-500 to-primary-300 bg-clip-text text-transparent box-decoration-clone">
            {data.name}
        </span>
    </h1>

    {#if data.user}
        <form class="mt-12" method="POST" action="/login?/logout">
            <button class="btn variant-ghost-error mx-auto block relative">Log out</button>
        </form>
    {/if}
	<!-- {#each data.display.content as display}
        <p>{display.name}</p>
    {/each} -->
    
    <!-- <div class="card">
        <section class="p-4">

        <nav class="list-nav">
                <h2 class="h2">Displays</h2>
                <ul>
                    {#each data.display.content as display}
                    <li>
                        <a href="/display/{display.uuid}">
                            <span class="badge-icon p-4 variant-soft-primary">💀</span>
                            <span class="flex-auto">{display.name}</span>
                        </a>
                    </li>
                    {/each}
                </ul>
            </nav>
        </section>
    </div> -->
</div>
