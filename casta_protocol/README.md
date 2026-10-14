# Casta Protocol

The Casta Protocol is a lightweight WebSocket-based protocol used by display clients to communicate with the Sasta backend. Any client that implements this protocol can act as an Asta display.

_(Note: The reference client is already built-in to Sasta via an HTMX site, but you can build your own custom clients using this protocol)._

## Connection

Connect to Sasta via WebSocket at the root or `/ws` endpoint:

```
ws://<sasta-host>:<port>/ws
```

## Handshake

Once connected, the client must initiate the handshake by sending a `Hello` payload containing its Display UUID.

### Client Sends: `Hello`

```json
{
  "type": "Hello",
  "data": {
    "uuid": "your-display-uuid",
    "htmx": false
  }
}
```

_Note: Set `htmx: true` if your client expects rendered HTML fragments rather than raw data (used by the reference client)._

### Server Responds: `Welcome`

If the UUID is recognized, the server responds with:

```json
{
  "type": "Welcome",
  "data": {
    "name": "name-of-display",
    "htmx_hash": "optional-hash-string"
  }
}
```

**What is `htmx_hash`?**
If the client connects with `"htmx": true`, the server includes an `htmx_hash` in the Welcome payload. This represents the current version of the backend HTMX client. The client should store this hash; if it ever receives a new/different hash on a future connection, it indicates the backend was updated, and the client should trigger a full page refresh.

## Receiving Content

After the handshake, Sasta will stream content commands to the client based on the active Schedule/Playlist.

### Payload: `Pending`

You will receive a `Pending` payload in two scenarios:
1. **Unknown UUID:** If Sasta does *not* recognize the display's UUID during the handshake, it will not drop the connection. Instead, it sends `Pending(true)` and waits. The moment an admin registers that UUID via the Gasta GUI or API, the server will proceed seamlessly.
2. **Empty Schedule:** If the display is registered but currently has no active playlist scheduled.

```json
{
  "type": "Pending",
  "data": true
}
```

Clients should clear the screen or show a standby indicator when receiving a Pending state.

### Payload: `Display`

When it's time to show a new item, the server sends a `Display` payload:

```json
{
  "type": "Display",
  "data": {
    "type": "Website",
    "data": {
      "content": "https://example.com"
    }
  }
}
```

The inner `type` can be one of the following:

- `Website`: Display a web page.
- `Text`: Display raw text.
- `Image`: Display an image.
- `PortableDocumentFormat`: Display a PDF.

In all cases, the `content` field will contain the corresponding URL or text.
