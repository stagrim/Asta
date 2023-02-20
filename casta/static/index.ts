// One issue here is that since vanilla js websocket cannot check for pings, it cannot disconnect itself if no pings are received for a long time. 
// Big problem, and if so is a solution to use another npm package for websocket instead?

import ReconnectingWebSocket from "reconnecting-websocket"

interface Payload {
    Display?: Content<Website>,
    Disconnected?: any,
}

interface Content<T> {
    type: "Website",
    data: T
}

interface Website {
    content: string
}

window.onload = () => {
    const website: HTMLIFrameElement = document.getElementById("website") as HTMLIFrameElement

    const image: HTMLImageElement = document.getElementById("image") as HTMLImageElement
    // const background_audio: HTMLIFrameElement = document.getElementById("background_audio") as HTMLIFrameElement
    
    let socket = new ReconnectingWebSocket(`ws://${location.host}/ws`)

    socket.onopen = () => {
        console.log("connected to socket")
    }

    const assign = (element: HTMLElement | null, src?: string) => element?.setAttribute("src", src ?? "")

    const display_image = (src: string) => {
        website?.classList.add("hidden")
        image?.classList.remove("hidden")
        assign(image, src)
    }

    const display_website = (href: string) => {
        console.log("Website with data: " +href +" received")
        image.classList.add("hidden")
        website.classList.remove("hidden")
        assign(website, href)
    }

    const display_disconnected_sasta = () => {
        display_image("/disconnected")
    }

    const display_disconnected_casta = () => {
        display_image("/disconnected")
    }

    socket.onclose = () => {
        display_disconnected_casta()
        console.log("disconnected to socket")
    }

    socket.onerror = err => {
        console.error(
            "Socket encountered error: ",
            err.message,
            "Closing socket"
        )
        socket.close()
    }
    

    socket.onmessage = event => {
        console.log(`[Debug] ${event.data}`)
        let payload: Payload = JSON.parse(event.data)
        
        if (payload.Disconnected) {
            display_disconnected_sasta()
        } else if (payload.Display) {
            const type = payload.Display.type
            if (type === "Website") {
                display_website(payload.Display.data.content)
            }
        }

            // if (data.background_audio != undefined && data.background_audio !== current_background_audio) {
            //     current_background_audio = data.background_audio
            //     assign(background_audio, data.background_audio)
            // }
    }
}
