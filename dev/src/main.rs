use itertools::Itertools;
use std::{fs::File, io::Write, net::TcpListener};

use indoc::{printdoc, writedoc};

fn main() {
    let (sasta_db_port, gasta_db_port, sasta_port) =
        get_available_ports(3).into_iter().collect_tuple().unwrap();

    let mut env_file = File::create(".env").unwrap();
    writedoc! {env_file, "
        SASTA_DB_PORT={sasta_db_port}
        GASTA_DB_PORT={gasta_db_port}
    ",}
    .unwrap();

    let mut env_file = File::create("../gasta/.env").unwrap();
    writedoc! {env_file, "
        SERVER_URL=http://127.0.0.1:{sasta_port}
        LDAP_URL=ldap://localhost
        REDIS_URL=redis://127.0.0.1:{gasta_db_port}
    "}
    .unwrap();

    let mut env_file = File::create("../sasta/.env").unwrap();
    writedoc! {env_file, "
        REDIS_URL=redis://127.0.0.1:{sasta_db_port}
        ADDRESS=0.0.0.0:{sasta_port}
    "}
    .unwrap();

    printdoc! {"\n
        Allocated ports for services

        Sasta db use port     {sasta_db_port}
        Gasta db use port     {gasta_db_port}
        Sasta server use port {sasta_port}

        Run 'docker compose up -d' here to start databases
        Change .env files and docker-compose.yaml files to switch ports
    "};
}

fn get_available_ports(n: usize) -> Vec<u16> {
    (8560..9000)
        .filter(|port| port_is_available(*port))
        .take(n)
        .collect()
}

fn port_is_available(port: u16) -> bool {
    // Only checks for tcp connections, check for any kind of answer
    match TcpListener::bind(("127.0.0.1", port)) {
        Ok(_) => true,
        Err(_) => false,
    }
}
