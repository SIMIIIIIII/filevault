use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
};

const DEFAULT_RESULTS_PATH: &str = "benches/results/project_wothout_tokio.csv";

#[derive(Clone)]
struct CsvRow {
    size_label: String,
    bytes_per_client: u64,
    clients: usize,
    total_bytes: u64,
    iteration: usize,
    elapsed_ms: f64,
    throughput_mib_s: f64,
}

fn main() {
    let csv_path = resolve_csv_path();

    match read_csv_rows(&csv_path) {
        Ok(rows) => print_summary(&csv_path, &rows),
        Err(error) => {
            eprintln!("Error: {error}");
            std::process::exit(1);
        }
    }
}

fn resolve_csv_path() -> PathBuf {
    env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_RESULTS_PATH))
}

fn read_csv_rows(path: &Path) -> Result<Vec<CsvRow>, String> {
    let content = fs::read_to_string(path)
        .map_err(|error| format!("unable to read {}: {error}", path.display()))?;

    let mut lines = content.lines();
    let header = lines.next().ok_or_else(|| "empty CSV file".to_string())?;

    if header.trim()
        != "size_label,bytes_per_client,clients,total_bytes,iteration,elapsed_ms,throughput_mib_s"
    {
        return Err(format!(
            "invalid CSV header: {header}. Expected: size_label,bytes_per_client,clients,total_bytes,iteration,elapsed_ms,throughput_mib_s"
        ));
    }

    let mut rows = Vec::new();

    for (index, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }

        let columns: Vec<&str> = line.split(',').collect();
        if columns.len() != 7 {
            return Err(format!(
                "invalid line {}: {} columns found instead of 7",
                index + 2,
                columns.len()
            ));
        }

        rows.push(CsvRow {
            size_label: columns[0].to_string(),
            bytes_per_client: parse_u64(columns[1], index + 2, "bytes_per_client")?,
            clients: parse_usize(columns[2], index + 2, "clients")?,
            total_bytes: parse_u64(columns[3], index + 2, "total_bytes")?,
            iteration: parse_usize(columns[4], index + 2, "iteration")?,
            elapsed_ms: parse_f64(columns[5], index + 2, "elapsed_ms")?,
            throughput_mib_s: parse_f64(columns[6], index + 2, "throughput_mib_s")?,
        });
    }

    if rows.is_empty() {
        return Err("no measurements found in the CSV".to_string());
    }

    Ok(rows)
}

fn parse_u64(value: &str, line: usize, column: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|error| format!("line {line}, column {column}: {error}"))
}

fn parse_usize(value: &str, line: usize, column: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|error| format!("line {line}, column {column}: {error}"))
}

fn parse_f64(value: &str, line: usize, column: &str) -> Result<f64, String> {
    value
        .parse::<f64>()
        .map_err(|error| format!("line {line}, column {column}: {error}"))
}

fn print_summary(path: &Path, rows: &[CsvRow]) {
    let mut grouped: BTreeMap<(usize, String), Vec<&CsvRow>> = BTreeMap::new();

    for row in rows {
        grouped
            .entry((row.clients, row.size_label.clone()))
            .or_default()
            .push(row);
    }

    println!("Performance results");
    println!("=========================");
    println!("Source: {}", path.display());
    println!("Loaded measurements: {}", rows.len());
    println!();

    for ((clients, size_label), group_rows) in grouped {
        let iterations = group_rows.len();
        let bytes_per_client = group_rows[0].bytes_per_client;
        let total_bytes = group_rows[0].total_bytes;

        let min_elapsed_ms = group_rows
            .iter()
            .map(|row| row.elapsed_ms)
            .fold(f64::INFINITY, f64::min);
        let max_elapsed_ms = group_rows
            .iter()
            .map(|row| row.elapsed_ms)
            .fold(f64::NEG_INFINITY, f64::max);
        let avg_elapsed_ms =
            group_rows.iter().map(|row| row.elapsed_ms).sum::<f64>() / iterations as f64;

        let min_throughput = group_rows
            .iter()
            .map(|row| row.throughput_mib_s)
            .fold(f64::INFINITY, f64::min);
        let max_throughput = group_rows
            .iter()
            .map(|row| row.throughput_mib_s)
            .fold(f64::NEG_INFINITY, f64::max);
        let avg_throughput = group_rows
            .iter()
            .map(|row| row.throughput_mib_s)
            .sum::<f64>()
            / iterations as f64;

        let max_iteration = group_rows
            .iter()
            .map(|row| row.iteration)
            .max()
            .unwrap_or(0);

        println!("Scenario: {clients} client(s), size {size_label}");
        println!(
            "  volume per client: {:.2} MiB | total volume: {:.2} MiB",
            bytes_per_client as f64 / (1024.0 * 1024.0),
            total_bytes as f64 / (1024.0 * 1024.0),
        );
        println!("  loaded iterations: {iterations} (max iteration: {max_iteration})");
        println!(
            "  time min/avg/max: {:.3} / {:.3} / {:.3} ms",
            min_elapsed_ms, avg_elapsed_ms, max_elapsed_ms,
        );
        println!(
            "  throughput min/avg/max: {:.2} / {:.2} / {:.2} MiB/s",
            min_throughput, avg_throughput, max_throughput,
        );
        println!();
    }
}
