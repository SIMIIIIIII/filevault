
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    net::TcpListener,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

use chrono::Utc;
use file_vault::{customer::Customer, server::Server};
use tempfile::tempdir;

const HOST: &str = "::1";
const SERVER_TIMEOUT: Duration = Duration::from_secs(1);
const CONNECT_RETRIES: usize = 20;
const CONNECT_RETRY_DELAY: Duration = Duration::from_millis(50);
const STARTUP_DELAY: Duration = Duration::from_millis(100);
const FILE_READY_TIMEOUT: Duration = Duration::from_secs(30);
const FILE_READY_POLL: Duration = Duration::from_millis(20);
const ITERATIONS_PER_SIZE: usize = 5;
const BENCHMARK_CLIENT_COUNTS: [usize; 2] = [1, 4];
const RESULTS_DIRECTORY: &str = "benches/results";
const RESULTS_BASENAME: &str = "project_wothout_tokio";

const BENCHMARK_SIZES: [(&str, usize); 3] = [
    ("1KB", 1_024),
    ("1MB", 1_024 * 1_024),
    ("100MB", 100 * 1_024 * 1_024),
];

struct ThroughputResult {
    label: &'static str,
    bytes: usize,
    clients: usize,
    iteration: usize,
    elapsed: Duration,
    throughput_mib_s: f64,
}

struct ThroughputSummary {
    average_elapsed: Duration,
    min_elapsed: Duration,
    max_elapsed: Duration,
    average_throughput_mib_s: f64,
    min_throughput_mib_s: f64,
    max_throughput_mib_s: f64,
}

struct ScenarioReport {
    label: &'static str,
    bytes: usize,
    clients: usize,
    iterations: usize,
    summary: ThroughputSummary,
}

struct BenchmarkArtifacts {
    latest_csv_path: PathBuf,
    timestamped_csv_path: PathBuf,
    latest_markdown_path: PathBuf,
    timestamped_markdown_path: PathBuf,
}

fn main() {
    println!("FileVault throughput benchmark");
    println!("============================");

    let mut all_results = Vec::new();
    let mut reports = Vec::new();

    for clients in BENCHMARK_CLIENT_COUNTS {
        println!("Scenario: {clients} parallel client(s)");

        for (label, bytes) in BENCHMARK_SIZES {
            let results = measure_transfers(label, bytes, clients, ITERATIONS_PER_SIZE);
            let summary = summarize_results(&results);

            println!(
                "{label}: {:.2} MiB per client, {:.2} MiB total, {} iterations",
                bytes as f64 / (1024.0 * 1024.0),
                (bytes * clients) as f64 / (1024.0 * 1024.0),
                ITERATIONS_PER_SIZE,
            );
            println!(
                "  aggregate throughput min/avg/max: {:.2} / {:.2} / {:.2} MiB/s",
                summary.min_throughput_mib_s,
                summary.average_throughput_mib_s,
                summary.max_throughput_mib_s,
            );
            println!(
                "  total time min/avg/max       : {:.3?} / {:.3?} / {:.3?}",
                summary.min_elapsed,
                summary.average_elapsed,
                summary.max_elapsed,
            );

            reports.push(ScenarioReport {
                label,
                bytes,
                clients,
                iterations: ITERATIONS_PER_SIZE,
                summary,
            });
            all_results.extend(results);
        }

        println!();
    }

    let artifacts = write_results_artifacts(&all_results, &reports);
    println!("CSV written to {}", artifacts.latest_csv_path.display());
    println!("Timestamped CSV written to {}", artifacts.timestamped_csv_path.display());
    println!("Markdown summary written to {}", artifacts.latest_markdown_path.display());
    println!(
        "Timestamped Markdown summary written to {}",
        artifacts.timestamped_markdown_path.display()
    );
}

fn measure_transfers(
    label: &'static str,
    bytes: usize,
    clients: usize,
    iterations: usize,
) -> Vec<ThroughputResult> {
    (1..=iterations)
        .map(|iteration| measure_transfer(label, bytes, clients, iteration))
        .collect()
}

fn measure_transfer(
    label: &'static str,
    bytes: usize,
    clients: usize,
    iteration: usize,
) -> ThroughputResult {
    let source_dir = tempdir().expect("unable to create source temp dir");
    let server_dir = tempdir().expect("unable to create server temp dir");
    let storage_root = server_dir.path().join("storage");
    let history_path = server_dir.path().join("history.log");

    fs::create_dir_all(&storage_root).expect("unable to create storage directory");

    let source_files: Vec<PathBuf> = (0..clients)
        .map(|client_index| {
            let source_file = source_dir
                .path()
                .join(format!("payload_{label}_client_{client_index}.bin"));
            create_payload(&source_file, bytes);
            source_file
        })
        .collect();

    let port = find_available_port();
    let server_handle = spawn_server(port, storage_root.clone(), history_path);

    thread::sleep(STARTUP_DELAY);

    let destination_files: Vec<PathBuf> = source_files
        .iter()
        .map(|source_file| storage_root.join(source_file.file_name().expect("missing file name")))
        .collect();

    let start = Instant::now();
    let client_threads: Vec<_> = source_files
        .into_iter()
        .map(|source_file| {
            thread::spawn(move || {
                let mut customer = connect_customer_with_retry(port);
                customer
                    .send_file(source_file.to_string_lossy().into_owned(), None)
                    .expect("file transfer failed");
            })
        })
        .collect();

    for client_thread in client_threads {
        client_thread.join().expect("client thread panicked");
    }

    wait_until_all_received(&destination_files, bytes, FILE_READY_TIMEOUT, FILE_READY_POLL);
    let elapsed = start.elapsed();

    server_handle
        .join()
        .expect("server thread panicked")
        .expect("server error");

    let total_bytes = bytes * clients;
    let throughput_mib_s = (total_bytes as f64 / (1024.0 * 1024.0)) / elapsed.as_secs_f64();

    ThroughputResult {
        label,
        bytes,
        clients,
        iteration,
        elapsed,
        throughput_mib_s,
    }
}

fn summarize_results(results: &[ThroughputResult]) -> ThroughputSummary {
    let elapsed_total = results
        .iter()
        .fold(Duration::ZERO, |accumulator, result| accumulator + result.elapsed);
    let average_elapsed = elapsed_total.div_f64(results.len() as f64);
    let min_elapsed = results
        .iter()
        .map(|result| result.elapsed)
        .min()
        .expect("missing minimum elapsed time");
    let max_elapsed = results
        .iter()
        .map(|result| result.elapsed)
        .max()
        .expect("missing maximum elapsed time");

    let throughput_total: f64 = results
        .iter()
        .map(|result| result.throughput_mib_s)
        .sum();
    let average_throughput_mib_s = throughput_total / results.len() as f64;
    let min_throughput_mib_s = results
        .iter()
        .map(|result| result.throughput_mib_s)
        .fold(f64::INFINITY, f64::min);
    let max_throughput_mib_s = results
        .iter()
        .map(|result| result.throughput_mib_s)
        .fold(f64::NEG_INFINITY, f64::max);

    ThroughputSummary {
        average_elapsed,
        min_elapsed,
        max_elapsed,
        average_throughput_mib_s,
        min_throughput_mib_s,
        max_throughput_mib_s,
    }
}

fn write_results_artifacts(
    results: &[ThroughputResult],
    reports: &[ScenarioReport],
) -> BenchmarkArtifacts {
    let output_directory = PathBuf::from(RESULTS_DIRECTORY);
    fs::create_dir_all(&output_directory).expect("unable to create benchmark results directory");

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let latest_csv_path = output_directory.join(format!("{RESULTS_BASENAME}.csv"));
    let timestamped_csv_path = output_directory.join(format!("{RESULTS_BASENAME}_{timestamp}.csv"));
    let latest_markdown_path = output_directory.join(format!("{RESULTS_BASENAME}.md"));
    let timestamped_markdown_path =
        output_directory.join(format!("{RESULTS_BASENAME}_{timestamp}.md"));

    write_results_csv(&latest_csv_path, results);
    write_results_csv(&timestamped_csv_path, results);
    write_markdown_summary(&latest_markdown_path, reports, results.len());
    write_markdown_summary(&timestamped_markdown_path, reports, results.len());

    BenchmarkArtifacts {
        latest_csv_path,
        timestamped_csv_path,
        latest_markdown_path,
        timestamped_markdown_path,
    }
}

fn write_results_csv(output_path: &Path, results: &[ThroughputResult]) {
    let mut file = fs::File::create(output_path).expect("unable to create benchmark CSV");

    writeln!(
        file,
        "size_label,bytes_per_client,clients,total_bytes,iteration,elapsed_ms,throughput_mib_s"
    )
    .expect("unable to write CSV header");

    for result in results {
        writeln!(
            file,
            "{},{},{},{},{},{:.3},{:.2}",
            result.label,
            result.bytes,
            result.clients,
            result.bytes * result.clients,
            result.iteration,
            result.elapsed.as_secs_f64() * 1_000.0,
            result.throughput_mib_s,
        )
        .expect("unable to write CSV row");
    }
}

fn write_markdown_summary(output_path: &Path, reports: &[ScenarioReport], total_measurements: usize) {
    let mut file = fs::File::create(output_path).expect("unable to create benchmark markdown");

    writeln!(file, "# FileVault Benchmark Results").expect("unable to write markdown title");
    writeln!(file).expect("unable to write markdown spacing");
    writeln!(file, "- Loaded measurements: {total_measurements}").expect("unable to write markdown header");
    writeln!(file, "- Iterations per scenario: {}", ITERATIONS_PER_SIZE)
        .expect("unable to write markdown header");
    writeln!(file).expect("unable to write markdown spacing");

    let mut grouped: BTreeMap<usize, Vec<&ScenarioReport>> = BTreeMap::new();
    for report in reports {
        grouped.entry(report.clients).or_default().push(report);
    }

    for (clients, group_reports) in grouped {
        writeln!(file, "## {} parallel client(s)", clients)
            .expect("unable to write markdown section title");
        writeln!(file).expect("unable to write markdown spacing");
        writeln!(
            file,
            "| Size | Volume/client | Total volume | Iterations | Time min/avg/max | Throughput min/avg/max |"
        )
        .expect("unable to write markdown table header");
        writeln!(
            file,
            "|---|---:|---:|---:|---:|---:|"
        )
        .expect("unable to write markdown table separator");

        for report in group_reports {
            writeln!(
                file,
                "| {} | {:.2} MiB | {:.2} MiB | {} | {:.3?} / {:.3?} / {:.3?} | {:.2} / {:.2} / {:.2} MiB/s |",
                report.label,
                report.bytes as f64 / (1024.0 * 1024.0),
                (report.bytes * report.clients) as f64 / (1024.0 * 1024.0),
                report.iterations,
                report.summary.min_elapsed,
                report.summary.average_elapsed,
                report.summary.max_elapsed,
                report.summary.min_throughput_mib_s,
                report.summary.average_throughput_mib_s,
                report.summary.max_throughput_mib_s,
            )
            .expect("unable to write markdown row");
        }

        writeln!(file).expect("unable to write markdown spacing");
    }
}

fn create_payload(path: &Path, bytes: usize) {
    let payload = vec![b'x'; bytes];
    fs::write(path, payload).expect("unable to write payload file");
}

fn find_available_port() -> u64 {
    let listener = TcpListener::bind((HOST, 0)).expect("unable to bind to an ephemeral port");
    let port = listener
        .local_addr()
        .expect("missing local address")
        .port();
    drop(listener);
    u64::from(port)
}

fn spawn_server(
    port: u64,
    storage_root: PathBuf,
    history_path: PathBuf,
) -> thread::JoinHandle<Result<(), file_vault::files_vault_errors::FileVaultError>> {
    thread::spawn(move || {
        let mut server = Server::from_with_paths(HOST.to_string(), port, storage_root, history_path);
        server.listening(SERVER_TIMEOUT)
    })
}

fn connect_customer_with_retry(port: u64) -> Customer {
    for attempt in 0..CONNECT_RETRIES {
        let customer = Customer::from(HOST.to_string(), port).expect("unable to create customer");
        if customer.is_connected() {
            return customer;
        }

        if attempt + 1 < CONNECT_RETRIES {
            thread::sleep(CONNECT_RETRY_DELAY);
        }
    }

    panic!("unable to connect to server on port {port}");
}

fn wait_until_all_received(paths: &[PathBuf], expected_size: usize, timeout: Duration, poll: Duration) {
    let start = Instant::now();

    while start.elapsed() < timeout {
        if paths.iter().all(|path| file_has_expected_size(path, expected_size)) {
            return;
        }

        thread::sleep(poll);
    }

    panic!("files were not fully received within {:?}", timeout);
}

fn file_has_expected_size(path: &Path, expected_size: usize) -> bool {
    fs::metadata(path)
        .map(|metadata| metadata.len() == expected_size as u64)
        .unwrap_or(false)
}