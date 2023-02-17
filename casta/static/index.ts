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
    const background_audio: HTMLIFrameElement = document.getElementById("background_audio") as HTMLIFrameElement

    // Ignore error, library is defined in index.html
    // TODO: import correctly?
    // let socket = new WebSocket(`ws://${location.host}/ws`)
    let socket = new ReconnectingWebSocket(`ws://${location.host}/ws`)
    

    socket.addEventListener("open", () => {
        console.log("connected to socket")
    })

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

    socket.addEventListener("close", () => {
        display_disconnected_casta()
        console.log("disconnected to socket")
    })

    socket.addEventListener('message', (event: MessageEvent<any>) => {
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
    });
}
