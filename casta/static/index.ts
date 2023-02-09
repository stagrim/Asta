window.onload = () => {
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

    const website = document.getElementById("website")
    const website2 = document.getElementById("website2")
    let first_website = true

    const image = document.getElementById("image")
    const background_audio = document.getElementById("background_audio")
    let current_background_audio: string = "";

    let socket = new WebSocket(`ws://${location.host}/ws`)

    socket.addEventListener("open", () => {
        console.log("connected to socket")
    })

    socket.addEventListener("ping", () => {
        console.log("Received ping from server")
    })

    socket.addEventListener("close", () => {
        console.log("disconnected to socket")
    })

    const assign = (element: HTMLElement, src?: string) => element.setAttribute("src", src ?? "")

    socket.addEventListener('message', (event) => {
        console.log(`[Debug] ${event.data}`)
        let payload: Payload = JSON.parse(event.data)
        
        if (website && background_audio && image) {
            if (payload.Disconnected) {
                console.log("Now it is Disconnected")

                website.classList.add("hidden")
                image.classList.remove("hidden")
                assign(image, "/disconnected")

            } else if (payload.Display) {
                const type = payload.Display.type
                if (type === "Website") {
                    const content = payload.Display.data.content
                    console.log("Website with data: " +content +" received")

                    image.classList.add("hidden")
                    website.classList.remove("hidden")

                    assign(website, content)
                }
            }

            // if (data.background_audio != undefined && data.background_audio !== current_background_audio) {
            //     current_background_audio = data.background_audio
            //     assign(background_audio, data.background_audio)
            // }
        } else {
            console.error("Document did not load correctly");
        }
    });
}
