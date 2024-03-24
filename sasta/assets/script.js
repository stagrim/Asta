document.addEventListener("htmx:wsOpen", (e) => {
    htmx.addClass(htmx.find("#disconnected"), "hidden")
    const ws = e.detail.socketWrapper
    const test = {
        uuid: uuid,
        hostname: "htmx-client",
        htmx: true,
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
