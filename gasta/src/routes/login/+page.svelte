<script lang="ts">
	import { enhance } from "$app/forms";
	import type { ActionData, PageData } from "./$types";
	import { toastStore } from "$lib/stores";

    export let data: PageData;

    export let form: ActionData;

    $: if (form?.msg) {
        $toastStore.trigger({
            message: form.msg,
            background: 'variant-filled-error',
            autohide: true,
            timeout: 5000,
        })
    }
    let formLoading = false;
</script>
{#if data?.banner }
    <div class="variant-filled-warning text-center">
        {data.banner}
    </div>
{/if}

<div class="flex justify-center">
    <form class="card m-4 max-w-4xl" method="POST" action="?/login" use:enhance={() => {
        formLoading = true;
        return async ({ update }) => {
            await update();
            formLoading = false;
        };
    }}>
        <section class="p-4">
            <label class="label mb-5">
                <span>username</span>
                <input name="username" class="input" type="text" value={ form?.username ?? "" } required />
            </label>

            <label class="label mb-5">
                <span>password</span>
                <input name="password" class="input" type="password" required />
            </label>

            <button class="btn variant-filled-primary" disabled={formLoading}>Log in</button>
        </section>
    </form>
</div>
