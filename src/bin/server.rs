use std::{env, path::PathBuf, time::Duration};

use file_vault::{
    cli_helpers::{
        RuntimeMode,
        ensure_runtime_directories,
        get_history_test,
        map_error,
        parse_host_port,
        parse_runtime_mode,
        runtime_directories
    },
    server::Server,
};

const LISTENER_TIMEOUT: Duration = Duration::from_secs(15 * 60);

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("[ERROR]: {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let (runtime_mode, args) = parse_runtime_mode(args);

    ensure_runtime_directories(runtime_mode).await?;

    let args = args.into_iter();

    let (host, port) = parse_host_port(args.collect())?;

    let mut server = if runtime_mode == RuntimeMode::Production {
        Server::from(host, port)
    } else {
        let (storage_root, log_root) = runtime_directories(RuntimeMode::Test);
        Server::from_with_paths(
            host,
            port,
            PathBuf::from(storage_root),
            PathBuf::from(get_history_test(log_root))
        )
    };
    server.listening_async(LISTENER_TIMEOUT).await.map_err(map_error)?;

    Ok(())
}
