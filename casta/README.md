# Client-side of Asta, aka Casta

Casta does currently consist of a server which handles api requests, and a view which receives updates from server using a websocket. To display Casta an external web browser is currently needed. This will be replaced with Electron or Tauri ([preferred once this is implemented](https://github.com/tauri-apps/tauri/issues/3478)) in the future.

## Build

To compile Typescript in view.

`npm install && npm run build`

To start server with optimizations (remove release flag for debug build). Replace `run` with `build` to only compile.

`cargo run --release`
