import { SERVER_URL } from "$env/static/private"
import { fail, type Actions, redirect } from "@sveltejs/kit"
import type { Display, Payload } from "../../app"

export const actions = {
    create: async ({ request }) => {
        const data = await request.formData()

        const name = data.get("name")
        const schedule = data.get("schedule")

        const res = await fetch(`${SERVER_URL}/api/display`, {
            method: "POST",
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify({ name, schedule })
        })

        if (res.ok) {
            const json: Payload<Display> = await res.json()
            throw redirect(303, `/display/${json.content[0].uuid}`);
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
