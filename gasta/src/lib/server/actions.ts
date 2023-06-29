import { env } from "$env/dynamic/private"
import { fail, redirect } from "@sveltejs/kit"
import type { Payload } from "../../api_bindings/read/Payload"

export type type = "Display" | "Playlist" | "Schedule"

interface Input {
    body: { [key: string]: any },
    type: type,
    data: FormData,
    uuid?: string
}

export const create = async ({ body, type, data }: Input) => {
    for (const key of Object.keys(body)) {
        const field = data.get(key)
        if (field) {
            body[key] = field.toString()
        } else {
            return fail(400, { message: `The field ${key} is empty` })
        }
    }

    const res = await fetch(`${env.SERVER_URL}/api/${type.toLocaleLowerCase()}`, {
        method: "POST",
        headers: {'Content-Type': 'application/json'},
        body: JSON.stringify(body)
    })

    const text = await res.text()
    let payload: Payload
    try {
        payload = JSON.parse(text)
    } catch {
        return fail(400, { message: text })
    }
    
    if (payload.type == type) {
        console.log(payload)
        return { redirect: `/${type.toLocaleLowerCase()}/${payload.content[0].uuid}`, message: `${type} Added` }
    } else if (payload.type == "Error") {
        return fail(400, { message: payload.content.message })
    }
}

export const update = async ({ body, data, type, uuid }: Input) => {
    if (!uuid) return fail(400, { message: `Missing Uuid` })

    for (const key of Object.keys(body)) {
        const field = data.get(key)
        if (field) {
            try {
                body[key] = JSON.parse(field.toString())
            } catch (e) {
                body[key] = field.toString()
            }
        } else {
            return fail(400, { message: `${key} field was empty` })
        }
    }

    console.log(JSON.stringify(body))

    const res = await fetch(`${env.SERVER_URL}/api/${type.toLocaleLowerCase()}/${uuid}`, {
        method: "PUT",
        headers: {'Content-Type': 'application/json'},
        body: JSON.stringify(body)
    })

    const text = await res.text()
    let payload: Payload
    try {
        payload = JSON.parse(text)
        console.log(payload)
    } catch {
        console.log(text)
        return fail(400, { message: text })
    }
    
    if (payload.type == type) {
        return { message: "Display updated" }
    } else if (payload.type == "Error") {
        return fail(400, { message: payload.content.message })
    } else {
        return fail(400, { message: text })
    }
}

export const delete_action = async (type: type, uuid?: string) => {
    if (!uuid) return fail(400, { message: `Missing Uuid` })

    const _ = await fetch(`${env.SERVER_URL}/api/${type.toLocaleLowerCase()}/${uuid}`, {
        method: "DELETE",
    })

    return { redirect: `/${type.toLocaleLowerCase()}`, message: `${type} Deleted` }
}
