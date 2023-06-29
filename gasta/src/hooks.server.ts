import { env } from "$env/dynamic/private"
import { browser, dev, building, version } from '$app/environment';

if (env.SERVER_URL) {
    console.log(`Listening for Server on ${env.SERVER_URL}`)
} else if (!building) {
    throw new Error("SERVER_URL environment variable is not defined, can't connect to Server")
}
