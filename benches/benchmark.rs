use std::{
    fmt::Write as _,
    path::PathBuf,
};

use chrono::Utc;
use file_vault::{customer::Customer, server::Server};
use tempfile::tempdir;
use tokio::{
    fs,
    net::TcpListener,
    task::JoinHandle,
    time::{sleep, Duration, Instant},
};

const HOST: &str = "::1";
const SERVER_TIMEOUT: Duration = Duration::from_secs(1);
const STARTUP_DELAY: Duration = Duration::from_millis(100);
const CONNECT_RETRY_DELAY: Duration = Duration::from_millis(50);
const CONNECT_RETRIES: usize = 20;
const FILE_READY_TIMEOUT: Duration = Duration::from_secs(30);
const FILE_READY_POLL: Duration = Duration::from_millis(20);
const ITERATIONS_PER_SIZE: usize = 5;
const CLIENT_COUNTS: [usize; 2] = [1, 4];
const SIZES: [(&str, usize); 3] = [
    ("1KB", 1_024),
    ("1MB", 1_024 * 1_024),
    ("100MB", 100 * 1_024 * 1_024),
];

struct ResultRow {
    label: &'static str,
    bytes: usize,
    clients: usize,
    iteration: usize,
    elapsed: Duration,
}

#[tokio::main]
async fn main() {
    let mut results = Vec::new();

    for clients in CLIENT_COUNTS {
        for (label, bytes) in SIZES {
            for iteration in 1..=ITERATIONS_PER_SIZE {
                let result = measure_transfer(label, bytes, clients, iteration).await;
                println!(
                    "{label}, {clients} client(s), iteration {iteration}: {:.2} MiB/s",
                    throughput_mib_s(&result)
                );
                results.push(result);
            }
        }
    }

    write_reports(&results).await;
}

async fn measure_transfer(
    label: &'static str,
    bytes: usize,
    clients: usize,
    iteration: usize,
) -> ResultRow {
    let source_dir = tempdir().expect("unable to create source directory");
    let server_dir = tempdir().expect("unable to create server directory");
    let storage_root = server_dir.path().join("storage");
    let history_path = server_dir.path().join("history.log");
    fs::create_dir_all(&storage_root)
        .await
        .expect("unable to create storage directory");

    let mut source_files = Vec::with_capacity(clients);
    for client_index in 0..clients {
        let path = source_dir
            .path()
            .join(format!("payload_{label}_{client_index}.bin"));
        fs::write(&path, vec![b'x'; bytes])
            .await
            .expect("unable to create payload");
        source_files.push(path);
    }

    let port = available_port().await;
    let server = spawn_server(port, storage_root.clone(), history_path);
    sleep(STARTUP_DELAY).await;

    let destinations: Vec<PathBuf> = source_files
        .iter()
        .map(|path| storage_root.join(path.file_name().expect("missing filename")))
        .collect();

    let started_at = Instant::now();
    let mut clients_tasks = Vec::with_capacity(clients);
    for source_file in source_files {
        clients_tasks.push(tokio::spawn(async move {
            let mut customer = connect_with_retry(port).await;
            customer
                .send_file(source_file.to_string_lossy().into_owned(), None)
                .await
                .expect("file transfer failed");
        }));
    }

    for client in clients_tasks {
        client.await.expect("client task panicked");
    }
    wait_until_received(&destinations, bytes).await;
    let elapsed = started_at.elapsed();

    server
        .await
        .expect("server task panicked")
        .expect("server error");

    ResultRow {
        label,
        bytes,
        clients,
        iteration,
        elapsed,
    }
}

fn spawn_server(
    port: u64,
    storage_root: PathBuf,
    history_path: PathBuf,
) -> JoinHandle<Result<(), file_vault::files_vault_errors::FileVaultError>> {
    tokio::spawn(async move {
        let mut server = Server::from_with_paths(HOST.to_string(), port, storage_root, history_path);
        server.listening_async(SERVER_TIMEOUT).await
    })
}

async fn available_port() -> u64 {
    let listener = TcpListener::bind((HOST, 0))
        .await
        .expect("unable to bind an ephemeral port");
    u64::from(listener.local_addr().expect("missing listener address").port())
}

async fn connect_with_retry(port: u64) -> Customer {
    for attempt in 0..CONNECT_RETRIES {
        let customer = Customer::from(HOST.to_string(), port)
            .await
            .expect("unable to create customer");
        if customer.is_connected() {
            return customer;
        }
        if attempt + 1 < CONNECT_RETRIES {
            sleep(CONNECT_RETRY_DELAY).await;
        }
    }
    panic!("unable to connect to server on port {port}");
}

async fn wait_until_received(paths: &[PathBuf], expected_size: usize) {
    let deadline = Instant::now() + FILE_READY_TIMEOUT;
    while Instant::now() < deadline {
        let mut complete = true;
        for path in paths {
            let size_matches = fs::metadata(path)
                .await
                .map(|metadata| metadata.len() == expected_size as u64)
                .unwrap_or(false);
            if !size_matches {
                complete = false;
                break;
            }
        }
        if complete {
            return;
        }
        sleep(FILE_READY_POLL).await;
    }
    panic!("files were not received within {FILE_READY_TIMEOUT:?}");
}

fn throughput_mib_s(result: &ResultRow) -> f64 {
    (result.bytes * result.clients) as f64 / (1024.0 * 1024.0) / result.elapsed.as_secs_f64()
}

async fn write_reports(results: &[ResultRow]) {
    let directory = PathBuf::from("benches/results");
    fs::create_dir_all(&directory)
        .await
        .expect("unable to create results directory");

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let csv = build_csv(results);
    let markdown = build_markdown(results);

    for (name, content) in [
        ("stats.csv".to_string(), csv.clone()),
        (format!("stats_{timestamp}.csv"), csv),
        ("stats.md".to_string(), markdown.clone()),
        (format!("stats_{timestamp}.md"), markdown),
    ] {
        fs::write(directory.join(name), content)
            .await
            .expect("unable to write benchmark report");
    }
}

fn build_csv(results: &[ResultRow]) -> String {
    let mut output = String::from("size_label,bytes_per_client,clients,total_bytes,iteration,elapsed_ms,throughput_mib_s\n");
    for result in results {
        writeln!(
            output,
            "{},{},{},{},{},{:.3},{:.2}",
            result.label,
            result.bytes,
            result.clients,
            result.bytes * result.clients,
            result.iteration,
            result.elapsed.as_secs_f64() * 1_000.0,
            throughput_mib_s(result),
        )
        .expect("writing to String cannot fail");
    }
    output
}

fn build_markdown(results: &[ResultRow]) -> String {
    let mut output = String::from("# FileVault Benchmark Results\n\n| Size | Clients | Iteration | Elapsed | Throughput |\n|---|---:|---:|---:|---:|\n");
    for result in results {
        writeln!(
            output,
            "| {} | {} | {} | {:.3?} | {:.2} MiB/s |",
            result.label,
            result.clients,
            result.iteration,
            result.elapsed,
            throughput_mib_s(result),
        )
        .expect("writing to String cannot fail");
    }
    output
}
