use tokio::{
    fs,
    io::{self, AsyncWriteExt, BufReader, AsyncBufReadExt},
    time::{Duration, sleep},
};

use std::{
    path::{Path, PathBuf}
};

use crate::{
    customer::Customer,
    files_vault_errors::FileVaultError,
    server::Server
};


const QUICK_TRANSFER_TIMEOUT: Duration = Duration::from_secs(2);
const LISTENER_TIMEOUT: Duration = Duration::from_secs(15 * 60);
const STARTUP_DELAY: Duration = Duration::from_millis(100);
const CONNECT_RETRY_DELAY: Duration = Duration::from_millis(50);
const CONNECT_RETRIES: usize = 40;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeMode {
    Production,
    Test,
}

pub fn parse_runtime_mode(args: Vec<String>) -> (RuntimeMode, Vec<String>) {
    let mut filtered = Vec::with_capacity(args.len());
    let mut runtime_mode = RuntimeMode::Production;

    for argument in args {
        match argument.as_str() {
            "--test" | "--tests" | "--test-mode" => {
                runtime_mode = RuntimeMode::Test},
            _ => filtered.push(argument),
        }
    }

    (runtime_mode, filtered)
}

pub fn runtime_directories(runtime_mode: RuntimeMode) -> (PathBuf, PathBuf) {
    match runtime_mode {
        RuntimeMode::Production => (
            PathBuf::from("log_files"),
            PathBuf::from("server_files"),
        ),
        RuntimeMode::Test => (
            PathBuf::from("tests/log_files"),
            PathBuf::from("tests/server_files"),
        ),
    }
}

pub async fn ensure_runtime_directories(runtime_mode: RuntimeMode) -> Result<(), String> {
    let (log_directory, server_directory) = runtime_directories(runtime_mode);

    fs::create_dir_all(&log_directory).await.map_err(|e| e.to_string())?;
    fs::create_dir_all(&server_directory).await.map_err(|e| e.to_string())?;

    Ok(())
}

pub async  fn run_legacy_mode(args: Vec<String>, runtime_mode: Option<RuntimeMode>) -> Result<(), String> {
    let args = strip_method_flag(args);

    match args.first().map(|value| value.as_str()) {
        Some("POST") => {
            let (host, port, _) = parse_host_port_from_slice(&args[1..])?;
            let remaining = args.get(3..).unwrap_or(&[]).to_vec();
            run_post_mode(
                host,
                port,
                runtime_mode,
                remaining,
                true
            ).await
        }
        Some("GET") => {
            let (host, port, _) = parse_host_port_from_slice(&args[1..])?;
            run_get_mode(host, port, runtime_mode).await
        }
        _ => Err(usage()),
    }
}

async fn run_post_mode(
    host: String,
    port: u64,
    runtime_mode: Option<RuntimeMode>,
    remaining: Vec<String>,
    join_server: bool,
) -> Result<(), String> {
    let (filename, root) = parse_post_request(remaining)?;
    let server_handle = spawn_server(
        host.clone(),
        port,
        QUICK_TRANSFER_TIMEOUT,
        runtime_mode
    );

    sleep(STARTUP_DELAY).await;

    let mut customer = connect_customer_with_retry(&host, port).await?;
    customer.send_file(filename, root).await.map_err(map_error)?;

    if join_server {
        server_handle
            .await
            .map_err(|_| "server thread panicked".to_string())?
            .map_err(map_error)?;
    }

    Ok(())
}

pub async fn run_get_mode(
    host: String,
    port: u64,
    runtime_mode: Option<RuntimeMode>
) -> Result<(), String> {
    let server_handle = spawn_server(
        host.clone(),
        port,
        LISTENER_TIMEOUT,
        runtime_mode
    );
    let mut customer = connect_customer_with_retry(&host, port).await?;

    interactive_loop(&mut customer, runtime_mode).await?;

    drop(server_handle);
    Ok(())
}

async fn interactive_loop(
    customer: &mut Customer,
    runtime_mode: Option<RuntimeMode>
) -> Result<(), String> {
    let stdin = io::stdin();
    let mut input = BufReader::new(stdin);

    loop {
        print!("filevault> ");
        io::stdout().flush().await.map_err(|e| e.to_string())?;

        let mut line = String::new();
        let bytes_read = input
            .read_line(&mut line)
            .await
            .map_err(|error| error.to_string())?;

        if bytes_read == 0 {
            break;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let tokens: Vec<String> = trimmed
            .split_whitespace()
            .map(|value| value.to_string())
            .collect();

        match tokens.first().map(|value| value.to_ascii_lowercase()).as_deref() {
            Some("exit") => break,
            Some("reconnect") => {
                reconnect_customer(customer, runtime_mode).await?;
            }
            Some("help") if tokens.get(1).map(|value| value.eq_ignore_ascii_case("prod")).unwrap_or(false) => {
                print_production_help();
            }
            Some("help") => {
                print_interactive_help();
            }
            Some("post") => {
                let (filename, root) = parse_post_request(tokens.into_iter().skip(1).collect())?;
                customer.send_file(filename, root).await.map_err(map_error)?;
            }
            _ => {
                print_interactive_help();
            }
        }
    }

    Ok(())
}

async fn reconnect_customer(
    customer: &mut Customer,
    runtime_mode: Option<RuntimeMode>
) -> Result<(), String> {
    if customer.connexion().await.is_ok() {
        return Ok(());
    }

    let _server_handle = spawn_server(
        customer.get_host(),
        customer.get_port(),
        LISTENER_TIMEOUT,
        runtime_mode
    );
    sleep(STARTUP_DELAY).await;
    customer.connexion().await.map_err(map_error)
}

async fn connect_customer_with_retry(host: &str, port: u64) -> Result<Customer, String> {
    let mut last_error = None;

    for attempt in 0..CONNECT_RETRIES {
        let customer = Customer::from(host.to_string(), port).await.map_err(map_error)?;

        if customer.is_connected() {
            return Ok(customer);
        }

        last_error = Some(format!("unable to connect to {host}:{port}"));

        if attempt + 1 < CONNECT_RETRIES {
            sleep(CONNECT_RETRY_DELAY).await;
        }
    }

    Err(last_error.unwrap_or_else(|| format!("unable to connect to {host}:{port}")))
}

fn spawn_server(
    host: String,
    port: u64,
    timeout: Duration,
    runtime_mode: Option<RuntimeMode>
) -> tokio::task::JoinHandle<Result<(), FileVaultError>> {

    let mut server = if runtime_mode.is_none() {
        Server::from(host, port)
    } else {
        let (log_root, storage_root) = runtime_directories(RuntimeMode::Test);
        Server::from_with_paths(
            host,
            port,
            PathBuf::from(storage_root),
            PathBuf::from(get_history_test(log_root))
        )
    };

    tokio::spawn(async move {
        server.listening_async(timeout).await
    })
}


fn strip_method_flag(args: Vec<String>) -> Vec<String> {
    if matches!(
        args.first().map(|value| value.as_str()),
        Some("--methode" | "methode" | "--method" | "method"| "--m")
    ) {
        return args.into_iter().skip(1).collect();
    }

    args
}

pub fn parse_host_port(args: Vec<String>) -> Result<(String, u64), String> {
    let (host, port, _) = parse_host_port_from_slice(&args)?;
    Ok((host, port))
}

pub fn parse_host_port_with_first(first: String, rest: Vec<String>) -> Result<(String, u64), String> {
    let mut args = Vec::with_capacity(rest.len() + 1);
    args.push(first);
    args.extend(rest);
    parse_host_port(args)
}

fn parse_host_port_from_slice(args: &[String]) -> Result<(String, u64, usize), String> {
    let host = args.first().ok_or_else(usage)?.clone();
    let port = args
        .get(1)
        .ok_or_else(usage)?
        .parse::<u64>()
        .map_err(|_| usage())?;

    Ok((host, port, 2))
}

fn parse_post_request(args: Vec<String>) -> Result<(String, Option<String>), String> {
    if args.is_empty() {
        return Err(usage());
    }

    let filename = sanitize_filename(&extract_value(&args, &["--filename", "filename"])
        .or_else(|| first_positional_value(&args))
        .ok_or_else(usage)?.to_string());

    if filename.is_err() {
        return Err(usage());
    }

    let root = extract_value(&args, &["--root", "root"]);

    Ok((filename.unwrap(), root))
}

fn extract_value(args: &[String], keys: &[&str]) -> Option<String> {
    for (index, value) in args.iter().enumerate() {
        if keys.iter().any(|key| value.eq_ignore_ascii_case(key)) {
            return args
                .get(index + 1)
                .filter(|candidate| !candidate.starts_with("--"))
                .cloned();
        }
    }

    None
}

fn first_positional_value(args: &[String]) -> Option<String> {
    args.iter().find(|value| !value.starts_with("--")).cloned()
}

pub fn map_error(error: FileVaultError) -> String {
    error.to_string()
}

fn print_interactive_help() {
    println!("Commands: POST filename [--root root], reconnect, help, help prod, exit");
}

fn print_production_help() {
    println!("Production usage:");
    println!("  cargo run -- hostname port");
    println!("  POST filename [--root root]");
    println!("  reconnect");
    println!("  help");
    println!("  help prod");
    println!("  exit");
}

pub fn usage() -> String {
    [
        "Usage:",
        "  cargo run -- --test hostname port",
        "  cargo run -- hostname port",
        "  cargo run -- server hostname port",
        "  cargo run -- filevault --methode POST hostname port --filename file [--root root]",
        "  cargo run -- filevault methode GET hostname port",
        "  cargo run -- customer --methode POST hostname port --filename file [--root root]",
        "  cargo run -- customer methode GET hostname port",
        "Interactive commands: POST filename [--root root], reconnect, help, help prod, exit",
    ]
    .join("\n")
}

fn sanitize_filename(name: &str) -> Result<String, FileVaultError> {
    let candidate = Path::new(name)
        .file_name()
        .ok_or(FileVaultError::IncorrectDataType)?
        .to_string_lossy()
        .into_owned();

    if candidate.is_empty() || candidate == ".." {
        return Err(FileVaultError::IncorrectDataType);
    }
    
    Ok(candidate)
}

pub fn get_history_test(log_root: PathBuf) -> String {
    log_root
        .join("history.log")
        .to_string_lossy()
        .to_owned()
        .to_string()
}