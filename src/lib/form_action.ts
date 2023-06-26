import { applyAction } from "$app/forms"
import { invalidateAll } from "$app/navigation"
import { toastStore } from "@skeletonlabs/skeleton"

// Set type is only to stop TS from complaining, could not find types for the function in SvelteKit
export const form_action: (_: any) => Promise<void> = async ({ result }) => {
    console.log(result)
    if (result.type === "success" || result.type === "redirect") {
        if (result.data?.message) {
            toastStore.trigger({
                message: result.data.message,
                background: 'variant-filled-success',
                timeout: 2000
            })
        }

        await invalidateAll()
        await applyAction(result)

    } else if (result.type === "failure") {
        toastStore.trigger({
            message: result.message,
            background: 'variant-filled-error',
            autohide: false
        })
    }
}

