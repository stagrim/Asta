import { SERVER_URL } from '$env/static/private';
import type { Content, Display, Payload, Playlist, Schedule } from '../app'
import type { LayoutServerLoad } from './$types';



export const load = (async (_) => {
    const get = async (api_route: string) => {
        const res: Payload<Content> = await fetch(`${SERVER_URL}/api/${api_route}`)
            .then(d => d.json()) satisfies Payload<Content>

        res.index = new Map()
        res.content.sort((a, b) => a.name.localeCompare(b.name))
        res.content.forEach((c, i) => {
            res.index.set(c.uuid, i)
        })
        return res
    }
    const display: Payload<Display> = await get('display') as Payload<Display>
    const schedule: Payload<Schedule> = await get('schedule') as Payload<Schedule>
    const playlist: Payload<Playlist> = await get('playlist') as Payload<Playlist>
    return {
        display,
        schedule,
        playlist
    }
}) satisfies LayoutServerLoad
