document.addEventListener("htmx:wsOpen", (e) => {
    htmx.addClass(htmx.find("#disconnected"), "hidden")
    const ws = e.detail.socketWrapper
    const test = {
        type: "Hello",
        data: {
            uuid,
            hostname: "htmx-client",
            htmx: true,
        },
    }
    ws.send(JSON.stringify(test))
})
;["htmx:wsClose", "htmx:wsError"].forEach((event) => {
    document.addEventListener(event, (_) => {
        htmx.removeClass(htmx.find("#disconnected"), "hidden")
    })
})

htmx.config.wsReconnectDelay = (_) => {
    return 5000
}

let hash
document.addEventListener("htmx:wsAfterMessage", (e) => {
    try {
        const message = JSON.parse(e.detail.message)
        if (message.type == "Welcome") {
            const welcome = message.data
            if (hash) {
                if (hash !== welcome.htmx_hash) {
                    console.log("Hashes were not identical, reloading...")
                    window.location.reload(true)
                    // Must be here, see https://github.com/wilsonzlin/minify-js/issues/21
                    return
                } else {
                    console.log("Hashes are identical")
                }
            } else {
                console.log("Saving hash " + welcome.htmx_hash)
                hash = welcome.htmx_hash
            }
        }
    } catch {}
})
