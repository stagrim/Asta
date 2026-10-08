# Gasta (GUI for Asta)

Gasta is the web-based graphical user interface for the Asta display management system.

Built with **SvelteKit** and powered by **Deno**, Gasta allows you to visually interact with the Sasta API to Create, Update, Read, and Delete Displays, Schedules, and Playlists without needing to manually write JSON payloads.

![Glittering Asta](img/glittering_asta.jpg 'Glittering Asta')

## 📸 Screenshots

![Gasta Display edit view](img/display.png 'Gasta Display edit view')

## ⚙️ Environment Configuration

Gasta relies on several environment variables to locate the Sasta backend and configure its OAuth provider (Authentik via Auth.js).

When developing locally, the `dev/` script automatically creates a `.env` file for you. For production or manual setup, create a `.env` file based on `.env.template`:

| Variable                | Required | Description                                                                                                                                                            |
| ----------------------- | -------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `SERVER_URL`            | **Yes**  | URL pointing to the Sasta backend API. Example: `http://127.0.0.1:8080`.                                                                                               |
| `AUTH_SECRET`           | **Yes**  | A random secret string used by Auth.js to encrypt session tokens.                                                                                                      |
| `AUTH_AUTHENTIK_ID`     | Optional | Authentik Client ID for OAuth login.                                                                                                                                   |
| `AUTH_AUTHENTIK_SECRET` | Optional | Authentik Client Secret.                                                                                                                                               |
| `AUTH_AUTHENTIK_ISSUER` | Optional | Authentik OIDC Endpoint URL.                                                                                                                                           |
| `OAUTH_GROUPS`          | Optional | Space-separated list of OAuth groups permitted to log in. If omitted, anyone can log in.                                                                               |
| `ORIGIN`                | Optional | The URL where the web UI will be publicly available (e.g., `https://gasta.example.com`). Required by SvelteKit when running the production node server behind a proxy. |
| `AUTH_TRUST_HOST`       | Optional | Set to `true` when running behind a reverse proxy to trust the forwarded host headers.                                                                                 |

## 🛠 Build & Run Instructions

For full instructions on building and running Gasta (including Docker and Local Development setups), please refer to the **[Root README](../README.md)**.

> **Note**: During local development, Gasta communicates with Sasta via typed API bindings. If the backend API changes, ensure Sasta is running and run `deno task gen:api` in this directory to regenerate the bindings.
