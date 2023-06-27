import type { Actions } from "@sveltejs/kit"
import type { CreateDisplay } from "../../api_bindings/create/CreateDisplay"
import { create } from "$lib/server/actions"

export const actions = {
    create: async ({ request }) => {
        const body: CreateDisplay = {
            name: "",
            schedule: ""
        }
        return await create({
            body,
            type: "Display",
            data: await request.formData()
        })
    }
} satisfies Actions;
