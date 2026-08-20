document.addEventListener("htmx:wsOpen", (e) => {
  htmx.addClass(htmx.find("#disconnected"), "hidden");
  const ws = e.detail.socketWrapper;
  const test = {
    type: "Hello",
    data: {
      uuid,
      hostname: "htmx-client",
      htmx: true,
    },
  };
  ws.send(JSON.stringify(test));
});
["htmx:wsClose", "htmx:wsError"].forEach((event) => {
  document.addEventListener(event, (_) => {
    htmx.removeClass(htmx.find("#disconnected"), "hidden");
  });
});

htmx.config.wsReconnectDelay = (_) => {
  return 5000;
};

let hash;
document.addEventListener("htmx:wsAfterMessage", (e) => {
  try {
    const message = JSON.parse(e.detail.message);
    if (message.type == "Welcome") {
      const welcome = message.data;
      if (hash) {
        if (hash !== welcome.htmx_hash) {
          console.log("Hashes were not identical, reloading...");
          window.location.reload(true);
          // Must be here, see https://github.com/wilsonzlin/minify-js/issues/21
          // Should probably switch to https://github.com/swc-project/swc
          return;
        } else {
          console.log("Hashes are identical");
        }
      } else {
        console.log("Saving hash " + welcome.htmx_hash);
        hash = welcome.htmx_hash;
      }
    }
  } catch {}
});

// PDF
pdfjsLib.GlobalWorkerOptions.workerSrc = "/assets/pdf.worker@3.11.174.min.js";

async function renderFirstPage(canvas, url) {
  const pdf = await pdfjsLib.getDocument(url).promise;
  const page = await pdf.getPage(1);

  const baseViewport = page.getViewport({ scale: 1 });
  const fitScale = Math.min(
    window.innerWidth / baseViewport.width,
    window.innerHeight / baseViewport.height,
  );
  const viewport = page.getViewport({ scale: fitScale });

  const context = canvas.getContext("2d");
  canvas.width = viewport.width;
  canvas.height = viewport.height;

  await page.render({
    canvasContext: context,
    viewport: viewport,
  }).promise;
}
