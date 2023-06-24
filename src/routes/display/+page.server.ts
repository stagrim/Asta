import { SERVER_URL } from "$env/static/private"
import { fail, type Actions, redirect } from "@sveltejs/kit"
import type { CreateDisplay } from "../../api_bindings/create/CreateDisplay"
import type { Payload } from "../../api_bindings/read/Payload"

export const actions = {
    create: async ({ request }) => {
        const data = await request.formData();
        
        const name = data.get("name")
        const schedule = data.get("schedule")
        if (!name || !schedule) {
            return fail(400, { message: "Fields were empty" })
        }
        const body: CreateDisplay = {
            name: name.toString(),
            schedule: schedule.toString()
        }

        const res = await fetch(`${SERVER_URL}/api/display`, {
            method: "POST",
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify(body)
        })

        const text = await res.text()
        try {
            const payload: Payload = JSON.parse(text)
            if (payload.type == "Display") {
                console.log(payload)
                throw redirect(303, `/display/${payload.content[0].uuid}`);
            } else if (payload.type == "Error") {
                return fail(400, { message: payload.content.message })
            }
        } catch {
            return fail(400, { message: text })
        }
    }
} satisfies Actions;
