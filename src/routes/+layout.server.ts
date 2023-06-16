import { SERVER_URL } from '$env/static/private';
import type { Content, Display, Payload, Playlist, Schedule, State } from '../app'
import type { LayoutServerLoad } from './$types';



export const load = (async (_) => {
    const get = async (api_route: string) => {
        const payload: Payload<Content> = await fetch(`${SERVER_URL}/api/${api_route}`)
            .then(d => d.json()) satisfies Payload<Content>

        const map = new Map()
        payload.content.forEach(c => map.set(c.uuid, c))

        const res: State<Content> = {
            type: payload.type,
            content: map
        }
        // payload.index = new Map()
        // payload.content.sort((a, b) => a.name.localeCompare(b.name))
        // res.content.forEach((c, i) => {
        //     res.index.set(c.uuid, i)
        // })
        return res
    }
    const display: State<Display> = await get('display') as State<Display>
    const schedule: State<Schedule> = await get('schedule') as State<Schedule>
    const playlist: State<Playlist> = await get('playlist') as State<Playlist>
    return {
        display,
        schedule,
        playlist
    }
}) satisfies LayoutServerLoad
