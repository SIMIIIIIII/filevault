use std::{env, path::PathBuf, time::Duration};

use file_vault::{
    api,
    cli_helpers::{
        RuntimeMode, ensure_runtime_directories, get_history_test, map_error, parse_host_port,
        parse_host_port_with_first, parse_runtime_mode, run_get_mode, run_legacy_mode,
        runtime_directories, usage,
    },
    db,
    grpc::run,
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

    let mut args = args.into_iter();
    let first = args.next().ok_or_else(usage)?;

    match first.as_str() {
        "api" => {
            let (host, port) = parse_host_port(args.collect())?;
            let pool = db::connexion_db(None).await.map_err(|e| {
                map_error(
                    file_vault::files_vault_errors::FileVaultError::DatabaseError(e.to_string()),
                )
            })?;
            let jwt_secret =
                env::var("JWT_SECRET").map_err(|_| "JWT_SECRET wasn't defined".to_string())?;
            api::run(host, port, pool, jwt_secret).await.map_err(|e| {
                map_error(
                    file_vault::files_vault_errors::FileVaultError::ConnectionError(e.to_string()),
                )
            })
        }

        "grpc" => {
            let (host, port) = parse_host_port(args.collect())?;
            let pool = db::connexion_db(None).await.map_err(|e| {
                map_error(
                    file_vault::files_vault_errors::FileVaultError::DatabaseError(e.to_string()),
                )
            })?;

            let jwt_secret =
                env::var("JWT_SECRET").map_err(|_| "JWT_SECRET wasn't defined".to_string())?;

            run::run(pool, host, port, jwt_secret).await?;

            Ok(())
        }

        "server" => {
            let (host, port) = parse_host_port(args.collect())?;

            let mut server = if runtime_mode == RuntimeMode::Production {
                Server::from(host, port)
            } else {
                let (storage_root, log_root) = runtime_directories(RuntimeMode::Test);
                Server::from_with_paths(
                    host,
                    port,
                    storage_root,
                    PathBuf::from(get_history_test(log_root)),
                )
            };

            server
                .listening_async(LISTENER_TIMEOUT)
                .await
                .map_err(map_error)?;
            Ok(())
        }
        "filevault" | "customer" => {
            run_legacy_mode(
                args.collect(),
                if runtime_mode == RuntimeMode::Production {
                    None
                } else {
                    Some(runtime_mode)
                },
            )
            .await
        }
        _ => {
            let (host, port) = parse_host_port_with_first(first, args.collect())?;
            run_get_mode(
                host,
                port,
                if runtime_mode == RuntimeMode::Production {
                    None
                } else {
                    Some(runtime_mode)
                },
            )
            .await
        }
    }
}
