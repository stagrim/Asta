window.onload = () => {
    interface Data {
        website?: string;
        image?: string;
        audio?: string;
        video?: string;
        background_audio?: string;
    }

    const website = document.getElementById("website")
    const image = document.getElementById("image")
    const background_audio = document.getElementById("background_audio")
    let current_background_audio: string = "";

    let socket = new WebSocket(`ws://${location.host}/ws`)

    socket.addEventListener("open", () => {
        console.log("connected to socket")
    })

    socket.addEventListener("close", () => {
        console.log("disconnected to socket")
    })

    const assign = (element: HTMLElement, src?: string) => element.setAttribute("src", src ?? "")

    socket.addEventListener('message', (event) => {
        console.log(event.data)
        let data: Data = JSON.parse(event.data)
        
        if (website && background_audio && image) {
            // TODO: implement behavior for the other keys
            // Hierarchy goes: website -> image -> video
            if (data.website) {
                data.image = undefined
                image.classList.add("hidden")
                website.classList.remove("hidden")
            } else if (data.image) {
                website.classList.add("hidden")
                image.classList.remove("hidden")
            }
            assign(website, data.website)
            assign(image, data.image)

            if (data.background_audio != undefined && data.background_audio !== current_background_audio) {
                current_background_audio = data.background_audio
                assign(background_audio, data.background_audio)
            }
        } else {
            console.error("Document did not load correctly");
        }
    });
}
