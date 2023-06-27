import type { Actions } from "@sveltejs/kit"
import { create } from "$lib/server/actions"
import type { CreateSchedule } from "../../api_bindings/create/CreateSchedule"

export const actions = {
    create: async ({ request }) => {
        const body: CreateSchedule = {
            name: "",
            playlist: ""
        }
        return await create({
            body,
            type: "Schedule",
            data: await request.formData()
        })
    }
} satisfies Actions;
