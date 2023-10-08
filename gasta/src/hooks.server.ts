import { env } from "$env/dynamic/private"
import { building } from '$app/environment';
import { redirect, type Handle } from "@sveltejs/kit";
import { session_display_name, session_username, valid_session } from "$lib/auth";

if (env.SERVER_URL) {
    console.log(`Listening for Server on ${env.SERVER_URL}`)
} else if (!building) {
    throw new Error("SERVER_URL environment variable is not defined, can't connect to Server")
}

if (env.LDAP_URL) {
    console.log(`Listening to LDAP on ${env.LDAP_URL}`)
} else if (!building) {
    throw new Error("LDAP_URL environment variable is not defined, can't connect to LDAP")
}

export const handle: Handle = async ({ event, resolve }) => {

    // Ensure browser security
    console.log(event.request.headers.get("SEC-CH-UA"));

    if (!event.url.pathname.startsWith('/not-supported') && event.request.headers.get("SEC-CH-UA")?.includes(`"Edge"`)) {
        throw redirect(303, "/not-supported")
    } else if (event.url.pathname.startsWith('/not-supported') && !event.request.headers.get("SEC-CH-UA")?.includes(`"Edge"`)) {
        throw redirect(303, "/")
    }

    const valid = valid_session(event.cookies.get('session-id')!, event.request.headers.get("User-Agent")!);
    if (!event.url.pathname.startsWith("/login") && !event.url.pathname.startsWith("/not-supported")) {
        if (valid) {
            event.locals.user = session_username(event.cookies.get('session-id')!)
            event.locals.name = session_display_name(event.cookies.get('session-id')!)
            // console.log("Valid req, will not redirect")
        } else {
            // console.log("Invalid req, will redirect to login")
            throw redirect(303, "/login")
        }
    } else if (event.url.pathname.startsWith("/login") && event.request.method === "GET") {
        // Get requests to login sites should redirect to start page if user session is valid.
        // Logout is a Post request to login, so only GET should be reflected
        if (valid) {
            throw redirect(303, "/")
        }
    }

    return resolve(event);
}