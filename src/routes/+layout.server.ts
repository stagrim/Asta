import { SERVER_URL } from '$env/static/private';
import type { Payload } from '../api_bindings/read/Payload'
import type { State } from '../app'
import type { LayoutServerLoad } from './$types';



export const load = (async (_) => {
    const get = async (api_route: string) => {
        const payload: Payload = await fetch(`${SERVER_URL}/api/${api_route}`)
            .then(d => d.json())

        if (payload.type == "Error") {
            console.error(`Error: ${payload}`)
            throw Error()
        }

        const map = new Map()
        payload.content.forEach(c => map.set(c.uuid, c))

        const res: State = {
            type: payload.type,
            content: map
        }
        return res
    }
    
    const display: State = await get('display')
    if (display.type != "Display") throw Error()
    const schedule: State = await get('schedule')
    if (schedule.type != "Schedule") throw Error()
    const playlist: State = await get('playlist')
    if (playlist.type != "Playlist") throw Error()
    return {
        display,
        schedule,
        playlist
    }
}) satisfies LayoutServerLoad
