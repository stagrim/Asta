import { SERVER_URL } from "$env/static/private"
import { redirect, type Actions, fail } from "@sveltejs/kit"

export const actions = {
    delete: async ({ params }) => {
        const uuid = params.uuid
        console.log(uuid)
        const test = await fetch(`${SERVER_URL}/api/display/${uuid}`, {
            method: "DELETE",
        })

        throw redirect(303, `/display`);
    },
    update: async ({ params, request }) => {
        const data = await request.formData();
        const uuid = params.uuid

        const name = data.get("name")
        const schedule = data.get("schedule")

        const res = await fetch(`${SERVER_URL}/api/display/${uuid}`, {
            method: "PUT",
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify({ name, schedule })
        })

        if (res.ok) {
            console.log(await res.json())
        } else {
            const text = await res.text()
            try {
                return fail(400, { message: JSON.parse(text).message })
            } catch {
                return fail(400, { message: text })
            }
        }
    } 
} satisfies Actions;
