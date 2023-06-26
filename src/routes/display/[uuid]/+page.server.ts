import { SERVER_URL } from "$env/static/private"
import type { Actions } from "@sveltejs/kit"
import type { UpdateDisplay } from "../../../api_bindings/update/UpdateDisplay"
import type { Payload } from "../../../api_bindings/read/Payload"
import { delete_action, update } from "../../../lib/server/actions"

export const actions = {
    delete: async ({ params }) => await delete_action("Display", params.uuid),
    update: async ({ params, request }) => {
        const body: UpdateDisplay = {
            name: "",
            schedule: ""
        }
        await update({
            body,
            data: await request.formData(),
            type: "Display",
            uuid: params.uuid
        })
    }
} satisfies Actions;
