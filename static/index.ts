// One issue here is that since vanilla js websocket cannot check for pings, it cannot disconnect itself if no pings are received for a long time. 
// Big problem, and if so is a solution to use another npm package for websocket instead?

import ReconnectingWebSocket from "reconnecting-websocket"

interface Payload {
    Display?: Content<ContentData>,
    Disconnected?: any,
    Hash?: string
}

interface Content<T> {
    type: "Website" | "Image" | "Text",
    data: T
}

interface ContentData {
    content: string
}

window.onload = () => {
    const website: HTMLIFrameElement = document.getElementById("website") as HTMLIFrameElement

    const image: HTMLImageElement = document.getElementById("image") as HTMLImageElement
    const text: HTMLDivElement = document.getElementById("text") as HTMLDivElement
    // const background_audio: HTMLIFrameElement = document.getElementById("background_audio") as HTMLIFrameElement
    const all_elements = [website, image, text]
    
    let socket = new ReconnectingWebSocket(`ws://${location.host}/ws`)

    socket.onopen = () => {
        console.log("connected to socket")
    }

    const assign_src = (element: HTMLElement, src?: string) => element.setAttribute("src", src ?? "")

    /** Hide all elements in all_elements except for given element element */
    const hide_other = (element: HTMLElement) =>
        all_elements.forEach(e => e.id === element.id ? e.classList.remove("hidden") : e.classList.add("hidden"))

    const display_image = (src: string) => {
        hide_other(image)
        assign_src(image, src)
    }

    const display_website = (href: string) => {
        console.log("Website with data: " +href +" received")
        hide_other(website)
        assign_src(website, href)
    }

    const display_text = (content: string) => {
        hide_other(text)
        text.innerHTML = content
    }

    const display_disconnected_sasta = () => {
        display_image("/disconnected.png")
    }

    const display_disconnected_casta = () => {
        display_image("/disconnected.png")
    }

    socket.onclose = () => {
        display_disconnected_casta()
        console.log("disconnected to socket")
    }
    

    let version_hash: string

    socket.onmessage = event => {
        console.log(`[Debug] ${event.data}`)
        let payload: Payload = JSON.parse(event.data)
        
        if (payload.Disconnected) {
            display_disconnected_sasta()
        } else if (payload.Hash) {
            if (!version_hash) {
                console.log(`[Hash] New hash received: "${payload.Hash}"`)
                version_hash = payload.Hash
            } else {
                if (version_hash === payload.Hash) {
                    console.log("[Hash] Sent hash matches with saved hash")
                } else {
                    console.log("[Hash] Saved hash does not match with sent one, refreshing page")
                    window.location.reload()
                }
            }
        } else if (payload.Display) {
            const type = payload.Display.type

            if (type === "Website") {
                const data = payload.Display.data as ContentData
                display_website(data.content)
            } else if (type === "Text") {
                const data = payload.Display.data as ContentData
                display_text(data.content)
            } else if (type === "Image") {
                const data = payload.Display.data as ContentData
                display_image(data.content)
            }
        }

            // if (data.background_audio != undefined && data.background_audio !== current_background_audio) {
            //     current_background_audio = data.background_audio
            //     assign(background_audio, data.background_audio)
            // }
    }
}
