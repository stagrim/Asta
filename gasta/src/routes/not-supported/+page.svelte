<script>

	import { Icon } from "svelte-awesome";
    import firefox from 'svelte-awesome/icons/firefox';
    import chrome from 'svelte-awesome/icons/chrome';
    import safari from 'svelte-awesome/icons/safari';

</script>

<div class="flex justify-center">
    <section class="p-4 card m-4 max-w-4xl">
        <h1 class="h1 py-4 font-semibold">Browser not Supported</h1>
        <div class="flex basis-0 gap-3 flex-col sm:flex-row">
            <div class="sm:w-1/2">
                <img src="/not_supported.png" class="rounded-md" alt="">
            </div>
            <div class="sm:w-1/2">
                <p>
                    Your browser is sadly not supported by the Asta system. Try updating to a newer version
                    or switch to browsers known to be working because of strong security and trust, like Firefox,
                    Chrome or Safari.
                </p>
                <br>
                <h3 class="h3">Download Links</h3>
                <div class="flex justify-evenly p-3">
                    <a target="_blank" href="https://www.mozilla.org/en-US/firefox/new/">
                        <Icon data={firefox} scale={5} />
                    </a>
                    <a target="_blank" href="https://www.google.com/chrome/">
                        <Icon data={chrome} scale={5} />
                    </a>
                    <a target="_blank" href="https://www.apple.com/safari/">
                        <Icon data={safari} scale={5} />
                    </a>
                </div>
            </div>
        </div>
    </section>
</div>