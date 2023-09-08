import { env } from "$env/dynamic/private";
import ldapjs, { type Client } from "ldapjs";
const { createClient } = ldapjs;
import pkg from "js-sha3";
import { building } from "$app/environment";
const { sha3_512 } = pkg;

export type Login = {
    result: "success",
    session_id: string
} | {
    result: "failure",
    msg: string
}

export interface Session {
    session_id: string,
    username: string,
    user_agent: string,
    expires: string,
    name: string
}

// TODO: Set setInterval to remove old sessions from map

const users: Map<string, Session> = new Map()

export const login = async (username: string, password: string, user_agent: string): Promise<Login> => {
    let res: Login
    let auth: Authenticate = await authenticate(username, password)
    
    if (auth.result === "success" || (process.env.NODE_ENV === "development" && username === "admin" && password === "admin")) {
        const message = `${user_agent}${Math.random()}${Date.now()}`
        let session: Session = {
            session_id: sha3_512(`${env.UUID5_NAMESPACE}${username}${message}`),
            user_agent,
            username,
            expires: "someday",
            name: auth.result === "success" ? auth.name : "Rosa Pantern"
        };
        if (users.has(session.session_id)) {
            res = {
                result: "failure",
                msg: "Session id is already in use??? (contact person responsible)"
            }
        } else {
            users.set(session.session_id, session)
            res = {
                result: "success",
                session_id: session.session_id
            }
        }
    } else {
        res = auth
    }
    return res
}

export const valid_session = (session_id: string, user_agent: string): boolean =>
    users.has(session_id) && users.get(session_id)?.user_agent === user_agent

export const session_username = (session_id: string): string =>
users.get(session_id)?.username ?? ""

export const session_display_name = (session_id: string): string =>
    users.get(session_id)?.name ?? ""

export const invalidate_session = (session_id: string) =>
    users.delete(session_id)

export type Authenticate = {
    result: "success",
    name: string
} | {
    result: "failure",
    msg: string
}

let client: Client

if (!building) {
    client = createClient({
        url: [env.LDAP_URL],
        timeout: 2000,
        connectTimeout: 2000,
        reconnect: true,
    });
    
    client.on('error', err => {
        console.debug({msg: 'connection failed, retrying', err});
    });
}


// (|(*group logic*)(*another group logic*))
const filter = `(|${['dsek.km', 'dsek.cafe', 'dsek.sex']
    .map(g => `(memberOf=cn=${g},cn=groups,cn=accounts,dc=dsek,dc=se)`)
    .join('')})`;

const authenticate = async (username: string, password: string): Promise<Authenticate> => {
    return new Promise(function (resolve) {
        // Oskar "badoddss" Stenberg was here
        if (!new RegExp("^[A-Za-z0-9]{6,10}(-s)?$").test(username)) {
            resolve({
                result: "failure",
                msg: `Ye ain't smart enough to remember yer stil-id, harr harr!`
            }) 
        }

        let res: string[] = []

        client.bind(`uid=${username},cn=users,cn=accounts,dc=dsek,dc=se`, password, (err, _) => {
            if (err) {
                console.log({err});
                client.unbind();
                resolve({
                    result: "failure",
                    msg: `Ye 'ave forgotten yer username or yer password`
                })
            } else {
                client.search(`uid=${username},cn=users,cn=accounts,dc=dsek,dc=se`, {  }, (searchError, searchResponse) => {
                    if (searchError) {
                        console.error('LDAP search error:', searchError);
                        client.unbind();
                        resolve({
                            result: "failure",
                            msg: `LDAP error1: ${searchError.message}`
                        })
                    }
                
                    searchResponse.on('searchEntry', (entry) => {
                        // The entry object contains information about the group
                        const display_name = entry.attributes.find(a => a.type === "givenName")?.values[0] ?? ""
                        res.push(display_name)
                    });
                
                    searchResponse.on('end', () => {
                        client.unbind();
                        if (res.length > 0) {
                            resolve({
                                result: "success",
                                name: res[0]
                            })
                        } else {
                            resolve({
                                result: "failure",
                                msg: "Ahoy there, matey! I be sorry to inform ye that ye don't have the proper authorization to be layin' eyes on the Asta web page. Arrr, it be guarded like a chest of precious booty, and only them with the right permissions can set their sights on it. Ye best be seekin' permission from the rightful owner or the webmaster if ye wish to gain access to that there treasure trove of information. Fair winds to ye on yer digital adventures, but for now, ye best be sailin' away from these waters. Arrr! 🏴‍☠️⚓🦜"
                            })
                        }
                    });

                    searchResponse.on('connectError', (err) => {
                        console.log(err)
                        resolve({
                            result: "failure",
                            msg: `LDAP error2: ${err.message}`
                        })
                    });
                });
            }
                
        });
    })
}
