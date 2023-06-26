import { SERVER_URL } from "$env/static/private"
import type { Actions } from "@sveltejs/kit"
import type { CreateDisplay } from "../../api_bindings/create/CreateDisplay"
import type { Payload } from "../../api_bindings/read/Payload"
import { create } from "$lib/server/actions"

export const actions = {
    create: async ({ request }) => {
        const body: CreateDisplay = {
            name: "",
            schedule: ""
        }
        return await create(body, "Display", await request.formData())
    }
} satisfies Actions;
