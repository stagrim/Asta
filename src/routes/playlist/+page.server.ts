import { SERVER_URL } from "$env/static/private"
import type { Actions } from "@sveltejs/kit"
import type { Payload } from "../../api_bindings/read/Payload"
import type { CreatePlaylist } from "../../api_bindings/create/CreatePlaylist"
import { create } from "$lib/server/actions"

export const actions = {
    create: async ({ request }) => {
        const body: CreatePlaylist = {
            name: ""
        }
        return await create(body, "Playlist", await request.formData())
    }
} satisfies Actions;
