use std::{
    path::Path,
    process::{self, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use file_vault::files::{open_file_read, open_file_write, write_in_file};
use tokio::{
    fs,
    io::AsyncReadExt,
    net::TcpListener,
    process::Command,
    sync::{Mutex, OnceCell},
    time::{timeout, Duration},
};

static FILE_COUNTER: AtomicU64 = AtomicU64::new(0);
const ROOT: &str = "tests";
const LOG_FILES: &str = "tests/log_files";
const SERVER_FILES: &str = "tests/server_files";
const PORT_RUN_FAILED: &str = "8011";
const HISTORY: &str = "history.log";
const PROCESS_TIMEOUT: Duration = Duration::from_secs(5);

static TEST_RUNTIME_LOCK: OnceCell<Mutex<()>> = OnceCell::const_new();

async fn create_and_fill_file(filename: String) {
    let mut file = open_file_write(&filename, false)
        .await
        .expect("unable to create test file");
    write_in_file(&mut file, b"test".to_vec())
        .await
        .expect("unable to write test file");
}

async fn remove_test_files(filename: &str) {
    let _ = fs::remove_file(get_file_path(filename.to_string(), ROOT.to_string())).await;
    let _ = fs::remove_file(get_file_path(
        filename.to_string(),
        SERVER_FILES.to_string(),
    ))
    .await;
    let _ = fs::remove_file(get_file_path(HISTORY.to_string(), LOG_FILES.to_string())).await;
}

fn get_file_path(filename: String, root: String) -> String {
    Path::new(&root)
        .join(filename)
        .to_string_lossy()
        .into_owned()
}

fn get_filename() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let index = FILE_COUNTER.fetch_add(1, Ordering::Relaxed);

    format!("test_{}_{}_{}.txt", process::id(), now, index)
}

async fn find_available_port() -> String {
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("unable to bind an ephemeral port");
    listener
        .local_addr()
        .expect("unable to read local address")
        .port()
        .to_string()
}

async fn run_interactive_session(commands: Vec<String>, port: &str) -> (bool, String, String) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_file_vault"))
        .arg("--test")
        .arg("localhost")
        .arg(port)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn binary");

    {
        let mut stdin = child.stdin.take().expect("stdin was piped");
        for command in commands {
            tokio::io::AsyncWriteExt::write_all(&mut stdin, format!("{command}\n").as_bytes())
                .await
                .expect("failed to write command");
        }
    }

    let output = timeout(PROCESS_TIMEOUT, child.wait_with_output())
        .await
        .expect("interactive process timed out")
        .expect("failed to wait on child");

    (
        output.status.success(),
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
    )
}

#[tokio::test]
async fn test_run_customer_success() {
    let _lock = TEST_RUNTIME_LOCK
        .get_or_init(|| async { Mutex::new(()) })
        .await
        .lock()
        .await;
    let filename = get_filename();
    let file_path = get_file_path(filename.clone(), ROOT.to_string());
    create_and_fill_file(file_path.clone()).await;
    let port = find_available_port().await;

    let output = Command::new(env!("CARGO_BIN_EXE_file_vault"))
        .args([
            "--test",
            "customer",
            "--methode",
            "POST",
            "127.0.0.1",
            &port,
            "--filename",
            &filename,
            "--root",
            ROOT,
        ])
        .output()
        .await
        .expect("failed to run binary");
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid utf8");
    assert!(stdout.contains(format!("[SERVER]: Listening on port {port}").as_str()));
    assert!(stdout.contains("[CLIENT]: file succefull sent!"));

    let mut sent_file = Vec::new();
    open_file_read(file_path)
        .await
        .expect("unable to open source file")
        .read_to_end(&mut sent_file)
        .await
        .expect("unable to read source file");
    let mut received_file = Vec::new();
    open_file_read(get_file_path(filename.clone(), SERVER_FILES.to_string()))
        .await
        .expect("unable to open received file")
        .read_to_end(&mut received_file)
        .await
        .expect("unable to read received file");
    assert_eq!(sent_file, received_file);

    remove_test_files(&filename).await;
}

#[tokio::test]
async fn test_run_fails_with_no_mode() {
    let output = Command::new(env!("CARGO_BIN_EXE_file_vault"))
        .args([
            "--test",
            "--methode",
            "POST",
            "localhost",
            PORT_RUN_FAILED,
            "--filename",
            "lib.rs",
            "--root",
            "src",
        ])
        .output()
        .await
        .expect("failed to run binary");
    assert!(!output.status.success());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("[ERROR]: Usage:"));
}

#[tokio::test]
async fn test_get_help() {
    let output = Command::new(env!("CARGO_BIN_EXE_file_vault"))
        .arg("help")
        .output()
        .await
        .expect("failed to run binary");
    assert!(!output.status.success());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("[ERROR]: Usage:"));
}

#[tokio::test]
async fn test_interactive_post_then_exit() {
    let _lock = TEST_RUNTIME_LOCK
        .get_or_init(|| async { Mutex::new(()) })
        .await
        .lock()
        .await;
    let filename = get_filename();
    let file_path = get_file_path(filename.clone(), ROOT.to_string());
    create_and_fill_file(file_path).await;
    let port = find_available_port().await;

    let (success, stdout, _) = run_interactive_session(
        vec![format!("POST {filename} --root {ROOT}"), "exit".to_string()],
        &port,
    )
    .await;
    assert!(success);
    assert!(stdout.contains("[CLIENT]: file succefull sent!"));
    remove_test_files(&filename).await;
}

#[tokio::test]
async fn test_interactive_help_then_exit() {
    let _lock = TEST_RUNTIME_LOCK
        .get_or_init(|| async { Mutex::new(()) })
        .await
        .lock()
        .await;
    let port = find_available_port().await;
    let (success, stdout, _) =
        run_interactive_session(vec!["help".to_string(), "exit".to_string()], &port).await;
    assert!(success);
    assert!(stdout.contains("Commands:"));
    assert!(stdout.contains("help"));
}

#[tokio::test]
async fn test_interactive_help_prod_then_exit() {
    let _lock = TEST_RUNTIME_LOCK
        .get_or_init(|| async { Mutex::new(()) })
        .await
        .lock()
        .await;
    let port = find_available_port().await;
    let (success, stdout, _) =
        run_interactive_session(vec!["help prod".to_string(), "exit".to_string()], &port).await;
    assert!(success);
    assert!(stdout.contains("Production usage:"));
    assert!(stdout.contains("help prod"));
}

#[tokio::test]
async fn test_interactive_unknown_command_then_exit() {
    let _lock = TEST_RUNTIME_LOCK
        .get_or_init(|| async { Mutex::new(()) })
        .await
        .lock()
        .await;
    let port = find_available_port().await;
    let (success, stdout, _) = run_interactive_session(
        vec!["unknown-command".to_string(), "exit".to_string()],
        &port,
    )
    .await;
    assert!(success);
    assert!(stdout.contains("Commands:"));
}
