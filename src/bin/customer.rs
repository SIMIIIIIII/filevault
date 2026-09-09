use std::env;

use file_vault::cli_helpers::{
    parse_host_port_with_first, run_client_get_mode, run_client_post_mode, usage,
};

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("[ERROR]: {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let first = args.next().ok_or_else(usage)?;
    let rest: Vec<String> = args.collect();

    let (host, port) = parse_host_port_with_first(first, rest.clone())?;
    let remaining: Vec<String> = rest.into_iter().skip(1).collect();

    match remaining
        .first()
        .map(|value| value.to_ascii_lowercase())
        .as_deref()
    {
        Some("send") => run_client_post_mode(host, port, remaining[1..].to_vec()).await,
        _ => run_client_get_mode(host, port).await,
    }
}
