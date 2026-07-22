use std::{
    env,
    time::Duration,
};

use file_vault::{
    server::Server,
    cli_helpers::{
        ensure_runtime_directories,
        parse_runtime_mode,
        usage,
        parse_host_port,
        map_error,
        run_legacy_mode,
        run_get_mode,
        parse_host_port_with_first
    }
};

const LISTENER_TIMEOUT: Duration = Duration::from_secs(15 * 60);


fn main() {
    if let Err(error) = run() {
        eprintln!("[ERROR]: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let (runtime_mode, args) = parse_runtime_mode(args);
    ensure_runtime_directories(runtime_mode)?;

    let mut args = args.into_iter();
    let first = args.next().ok_or_else(usage)?;

    match first.as_str() {
        "server" => {
            let (host, port) = parse_host_port(args.collect())?;
            let mut server = Server::from(host, port);
            server.listening(LISTENER_TIMEOUT).map_err(map_error)?;
            Ok(())
        }
        "filevault" | "customer" => run_legacy_mode(args.collect()),
        _ => {
            let (host, port) = parse_host_port_with_first(first, args.collect())?;
            run_get_mode(host, port)
        }
    }
}