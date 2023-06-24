import { SERVER_URL } from "$env/static/private"
import { redirect, type Actions, fail } from "@sveltejs/kit"
import type { UpdateDisplay } from "../../../api_bindings/update/UpdateDisplay"
import type { Payload } from "../../../api_bindings/read/Payload"

export const actions = {
    delete: async ({ params }) => {
        const uuid = params.uuid
        console.log(uuid)
        const _ = await fetch(`${SERVER_URL}/api/display/${uuid}`, {
            method: "DELETE",
        })

        throw redirect(303, `/display`);
    },
    update: async ({ params, request }) => {
        const data = await request.formData();
        const uuid = params.uuid
        
        const name = data.get("name")
        const schedule = data.get("schedule")
        if (!name || !schedule) {
            return fail(400, { message: "Fields were empty" })
        }
        const body: UpdateDisplay = {
            name: name.toString(),
            schedule: schedule.toString()
        }

        const res = await fetch(`${SERVER_URL}/api/display/${uuid}`, {
            method: "PUT",
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify(body)
        })

        const text = await res.text()
        try {
            const payload: Payload = JSON.parse(text)
            if (payload.type == "Display") {
                console.log(payload)
            } else if (payload.type == "Error") {
                return fail(400, { message: payload.content.message })
            }
        } catch {
            return fail(400, { message: text })
        }
    } 
} satisfies Actions;
