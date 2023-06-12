<script lang="ts">
    import type { PageData } from './$types';

    // export let data: PageData;
    // console.log(data.display)
</script>



<div class="container mx-auto p-8 space-y-8">
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
