# Client-side of Asta, aka Casta

Casta currently consist of a server which handles api requests, and a view which receives updates from server using a websocket. To display Casta an external web browser is currently needed. This will be replaced with Fasta (front end for Casta) using Electron or Tauri ([preferred once this is implemented](https://github.com/tauri-apps/tauri/issues/3478)) in the future.

## Build

To compile Typescript in view.

`npm install && npm run build`

To start server with optimizations (remove release flag for debug build). Replace `run` with `build` to only compile.

`cargo run --release`

## Run

Casta will provide an api server and a view hosted on specified port. Casta (and build docker image) does not *yet* provide a way to view the hosted site in and of itself. Fasta (not in built docker image) is a super simple electron window (for now) which displays the view which Casta hosts. Another way of viewing the hosted site would be using the kiosk mode in popular browsers:

`chromium --kiosk --autoplay-policy=no-user-gesture-required "127.0.0.1:<port>"`

(use alt+f4 or equivalent if you are unable to exit kiosk mode)

`firefox --kiosk --private-window "127.0.0.1:<port>"`

### Autoplay video and audio on Firefox
On firefox a policy.json file must be created to allow audio autoplay ([see](https://github.com/mozilla/policy-templates/blob/master/README.md)). This file is placed in `/etc/firefox/policies/` on linux to set is system-wide.

Script to create correct file on linux:
```bash
sudo mkdir -p /etc/firefox/policies/ && echo "{
  \"policies\": {
    \"Permissions\": {
      \"Autoplay\": {
        \"Default\": \"allow-audio-video\",
      }
    }
  }
}" | sudo tee /etc/firefox/policies/policies.json
```
