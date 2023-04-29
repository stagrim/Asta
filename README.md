# Client-side of Asta, aka Casta

Casta currently consist of a server which handles api requests, and a view which receives updates from server using a websocket. To display Casta an external web browser is currently needed. This will be replaced with Fasta (front end for Casta) using Electron or Tauri ([preferred once this is implemented](https://github.com/tauri-apps/tauri/issues/3478)) in the future.

![Child Asta](img/child_asta.jpg "Child Asta")

## Build

To compile Typescript in view.

`npm install && npm run build`

To start server with optimizations (remove release flag for debug build). Replace `run` with `build` to only compile.

`cargo run --release`

## Build Docker image

### Build for multiple platforms and push all to repo

```bash
docker buildx b --platform linux/amd64,linux/arm/v7 --push -t imagerepo.dsek.se/casta .
# May need to create new builder using `docker buildx create --use` before build command
```

### Load to system

```bash
docker buildx build --load -t casta .
# `--load` outputs the build as a docker image which is imported to the system. 
```


### Upload image build for raspberry pi

```bash
docker buildx build --platform linux/arm/v7 --push -t imagerepo.dsek.se/casta .
# `--push` outputs the build as a registry and pushes it to online repo based on specified tag
```

## Expected Server Requests

Upon connecting to websocket server Casta expects to be given a name like:

```json
{
  "name": "<name>"
}
```

Casta also expects server to contiguously get pinged, and will close the connection and attempt a reconnect if not having received a message within 10 seconds.

During a connection and after having received a name Casta listens and updates view by request from server like:

```json
{
  "display": {
    "type": "<TYPE>",
    "data": { ... },
  }
}
```

Defined values for type are described in [SastaConfDoc.md](../SastaConfDoc.md#Items) with object data changing depending on type.

Defined values for type in Casta

| TYPE | data |
|---|---|
| WEBSITE | `{ "content": "<Website to be shown>" }`
| TEXT | `{ "content": "<Text to be shown>" }`
| IMAGE | `{ "content": "<Image to be shown>" }`

Example to make Casta display the website https://dsek.se/:

```json
{
  "display": {
    "type": "WEBSITE",
    "data": {
      "content": "https://dsek.se/"
    },
  }
}
```

## Run
### Casta

To run the server from source cargo is used. Use `cargo run` with or without the release flag to build with or without optimizations. Running the docker image and connecting to the server running on the same system can be done with

`docker run --network host --env-file <env file> -p <local port>:3000 casta`

### Browser front-end

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

### Example Start Script
```bash
#!/bin/bash

# Starts a container called watchtower which will keep Casta updated when running
docker run -d --name watchtower \
  -v /var/run/docker.sock:/var/run/docker.sock \
  containrrr/watchtower \
  casta

# Starts Casta
docker run --env-file /home/pi/casta.env \
  --name casta -p 3000:3000 -d \
  imagerepo.dsek.se/casta:latest

# Starts chromium which will display Casta
chromium-browser --kiosk \
  --autoplay-policy=no-user-gesture-required \
  --enable-features=OverlayScrollbar,OverlayScrollbarFlashAfterAnyScrollUpdate,\
  OverlayScrollbarFlashAfterAnyScrollUpdate,OverlayScrollbarFlashWhenMouseEnter \
  127.0.0.1:3000
```
