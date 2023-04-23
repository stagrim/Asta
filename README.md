# Server-side of Asta, aka Sasta

Sasta consists of an API to get and update current state, a WebSocket connection to handle incoming connections from Clients (Casta) and a scheduling system to specify which playlist is active at which times using the cron syntax for start and end times.

![Smiling Asta](img/smiling_asta.jpg "Smiling Asta")

# Build

`cargo build --release`

# Run

`cargo run --release`

# Config file structure

```json
{
    "screens": { ... },
    "playlists": { ... },
    "schedules": { ... },
}
```

| Parameter | Type | Description|
|---|---|---|
| screens | Screen object | See [Screens](#Screens)
| playlists | playlists object | See [Playlists](#Playlists)
| schedules | schedules object | See [Schedules](#Schedules)

## Screens

```json
"<uuid>": {
    "name": "<name>",
    "schedule": "<uuid>",
}
```

| Parameter | Type | Description|
|---|---|---|
| key | string | uuid to identify screen send from Casta
| name | string | name of display to send to Casta
| schedule | string | uuid to identify schedule to use for screen

Example:
```json
"631f6175-6829-4e16-ad7f-cee6105f4c39": {
    "name": "Kotte",
    "schedule": "7fbd376b-a92a-40c2-8d20-0c67bfcc350d"
}
```

## Playlists

```json
"<uuid>": {
    "name": "<name>",
    "items": [ ... ],
}
```

| Parameter | Type | Description|
|---|---|---|
| key | string | uuid to identify playlist
| name | string | Playlist name
| items | Array<Item> | See [items](#Items)

Example:
```json
"d8486510-7931-43cc-9f86-e7c67a827ec0": {
    "name": "eventsAndChill",
    "items": [ ... ],
}
```

### Items

```json
{
    "name": "<name>",
    "type": "<type>",
    "settings": { ... }
}
```

| Parameter | Type | Description|
|---|---|---|
| name | string | Item name
| type | "IMAGE" / "TEXT" / "WEBSITE" | type determines which type of object settings will be
| settings | object | See [settings](#settings)

Example:
```json
{
    "name": "Nollning2020",
    "type": "IMAGE",
    "settings": { ... }
}
```

#### IMAGE

```json
"src": "https://www.dsek.se/images/hero-image.jpg",
"duration": 60
```

| Parameter | Type | Description|
|---|---|---|
| src | string | source to image
| duration | number | Duration in seconds of how long item will be shown

#### TEXT

```json
"text": "https://dsek.se/",
"duration": 60
```

| Parameter | Type | Description|
|---|---|---|
| text | string | text to be shown
| duration | number | Duration in seconds of how long item will be shown

#### WEBSITE

```json
"url": "https://dsek.se/",
"duration": 60
```

| Parameter | Type | Description|
|---|---|---|
| url | string | URL to website
| duration | number | Duration in seconds of how long item will be shown

## Schedules

```json
"<uuid>": {
    "name": "<name>",
    "scheduled": [ ... ],
    "playlist": "<uuid>",
}
```

| Parameter | Type | Description|
|---|---|---|
| key | string | uuid to identify schedule from display
| name | string | name of schedule
| scheduled | array? | see [scheduled](#scheduled), parameter is optional. Scheduled playlists in array is prioritized by placement in array, where first index holds the highest priority.
| playlist | string | uuid to identify playlist to use for screen

Examples:

Without scheduled playlists
```json
"7fbd376b-a92a-40c2-8d20-0c67bfcc350d": {
    "name": "Kotte",
    "playlist": "d8486510-7931-43cc-9f86-e7c67a827ec0"
}
```

With scheduled playlists
```json
"7fbd376b-a92a-40c2-8d20-0c67bfcc350d": {
    "name": "Kotte",
    "scheduled": [
        {
            "playlist": "ac0b0496-8b53-48c1-94d1-74683c3ae068",
            "start": "0 0 10 * * * *",
            "end": "0 0 13 * * * *"
        }
    ],
    "playlist": "d8486510-7931-43cc-9f86-e7c67a827ec0"
}
```

### Scheduled

```json
{
    "playlist": "<uuid>",
    "start": "0 0 10 * * * *",
    "end": "0 0 13 * * * *"
}
```

| Parameter | Type | Description|
|---|---|---|
| playlist | string | uuid to playlist active at specified times
| start | string | Cron expression stating when playlist should become active
| end | string | Cron expression stating when playlist should become inactive