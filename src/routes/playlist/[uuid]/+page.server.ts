import { SERVER_URL } from "$env/static/private"
import type { Actions } from "@sveltejs/kit"
import { delete_action, update } from "../../../lib/server/actions"
import type { UpdatePlaylist } from "../../../api_bindings/update/UpdatePlaylist"

const type = "Playlist"

export const actions = {
    delete: async ({ params }) => await delete_action(type, params.uuid),
    update: async ({ params, request }) => {
        const body: UpdatePlaylist = {
            name: "",
            items: []
        }
        await update({
            body,
            data: await request.formData(),
            type,
            uuid: params.uuid
        })
    }

} satisfies Actions;
