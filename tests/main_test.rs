use std::{
    fs,
    io::{Read, Write},
    path::Path,
    process::{self, Command, Stdio},
    sync::{atomic::{AtomicU64, Ordering}, mpsc},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH}
};

use file_vault::files::{open_file_read, open_file_write, write_in_file};

static FILE_COUNTER: AtomicU64 = AtomicU64::new(0);
const ROOT: &str = "tests";
const LOG_FILES: &str = "tests/log_files";
const SERVER_FILES: &str = "tests/server_files";
const PORT_RUN_SUCCES: &str = "8010";
const PORT_RUN_FAILED: &str = "8011";
const PORT_INTERACTIVE: &str = "8012";
const HISTORY: &str = "history.log";


fn create_and_fill_file(filename: String) {
    let get_file = open_file_write(filename.as_str(), false);
    assert!(get_file.is_ok());

    let mut file = get_file.unwrap();
    let bytes: Vec<u8> = "test".bytes().collect();

    assert!(write_in_file(&mut file, bytes.clone()).is_ok());
    drop(file);
}

fn remove_test_files(filename: &str) {
    let _ = fs::remove_file(get_file_path(filename.to_string(), ROOT.to_string()));
    let _ = fs::remove_file(get_file_path(filename.to_string(), SERVER_FILES.to_string()));
    let _ = fs::remove_file(get_file_path(HISTORY.to_string(), LOG_FILES.to_string()));
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
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let idx = FILE_COUNTER.fetch_add(1, Ordering::Relaxed);

    format!("test_{}_{}_{}.txt", process::id(), now, idx)
}

fn run_interactive_session(commands: Vec<String>, port: &str) -> (bool, String, String) {
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
        let stdin = child.stdin.as_mut().expect("stdin was piped");
        for command in commands {
            stdin.write_all(format!("{command}\n").as_bytes()).unwrap();
        }
    }

    let child_id = child.id();
    let (tx, rx) = mpsc::channel();
    let watchdog = thread::spawn(move || {
        if rx.recv_timeout(Duration::from_secs(5)).is_err() {
            let _ = Command::new("kill").args(["-9", &child_id.to_string()]).status();
        }
    });

    let output = child.wait_with_output().expect("failed to wait on child");
    let _ = tx.send(());
    let _ = watchdog.join();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();

    (output.status.success(), stdout, stderr)
}

#[test]
fn test_run_customer_success() {
    let filename = get_filename();
    let file_path = get_file_path(filename.clone(), ROOT.to_string());

    create_and_fill_file(file_path.clone());

    let output = Command::new(env!("CARGO_BIN_EXE_file_vault"))
        .arg("--test")
        .arg("customer")
        .arg("--methode")
        .arg("POST")
        .arg("localhost")
        .arg(PORT_RUN_SUCCES)
        .arg("--filename")
        .arg(filename.clone())
        .arg("--root")
        .arg(ROOT)
        .output()
        .expect("failed to run binary");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid utf8");

    assert!(stdout.contains(format!("[SERVER]: Listening on port {PORT_RUN_SUCCES}").as_str()));
    assert!(stdout.contains("[CLIENT]: file succefull sent!"));

    let get_sent_file = open_file_read(file_path.clone());
    assert!(get_sent_file.is_ok());
    
    let mut sent_file: Vec<u8> = Vec::new();
    assert!(get_sent_file.unwrap().read_to_end(&mut sent_file).is_ok());

    let get_received_file = open_file_read(
        get_file_path(filename.clone(),
        SERVER_FILES.to_string())
    );
    assert!(get_received_file.is_ok());

    let mut received_file: Vec<u8> = Vec::new();
    assert!(get_received_file.unwrap().read_to_end(&mut received_file).is_ok());

    assert_eq!(sent_file, received_file);

    remove_test_files(&filename);
}

#[test]
fn test_run_fails_with_no_mode() {
    let output = Command::new(env!("CARGO_BIN_EXE_file_vault"))
        .arg("--test")
        .arg("--methode")
        .arg("POST")
        .arg("localhost")
        .arg(PORT_RUN_FAILED)
        .arg("--filename")
        .arg("lib.rs")
        .arg("--root")
        .arg("src")
        .output()
        .expect("failed to run binary");

    assert!(!output.status.success());

    let stdout = String::from_utf8(output.stderr).expect("stdout should be valid utf8");

    assert!(stdout.contains("[ERROR]: Usage:"));
}

#[test]
fn test_get_help() {
    let output = Command::new(env!("CARGO_BIN_EXE_file_vault"))
        .arg("help")
        .output()
        .expect("failed to run binary");

    assert!(!output.status.success());

    let stdout = String::from_utf8(output.stderr).expect("stdout should be valid utf8");

    assert!(stdout.contains("[ERROR]: Usage:"));
}



#[test]
fn test_interactive_post_then_exit() {
    let filename = get_filename();
    let file_path = get_file_path(filename.clone(), ROOT.to_string());

    create_and_fill_file(file_path.clone());

    let commands = vec![
        format!("POST {filename} --root {ROOT}"),
        "exit".to_string(),
    ];

    let (success, stdout, _) = run_interactive_session(commands, PORT_INTERACTIVE);

    assert!(success);
    assert!(stdout.contains("[CLIENT]: file succefull sent!"));

    remove_test_files(&filename);
}

#[test]
fn test_interactive_help_then_exit() {
    let (success, stdout, stderr) = run_interactive_session(
        vec!["help".to_string(),
        "exit".to_string()],
        PORT_INTERACTIVE
    );
    println!("STDOUT:\n{stdout}\nSTDERR:\n{stderr}");

    assert!(success);
    assert!(stdout.contains("Commands:"));
    assert!(stdout.contains("help"));
}

#[test]
fn test_interactive_help_prod_then_exit() {
    let (success, stdout, stderr) = run_interactive_session(
        vec!["help prod".to_string(),
        "exit".to_string()],
        PORT_INTERACTIVE
    );
    println!("STDOUT:\n{stdout}\nSTDERR:\n{stderr}");

    assert!(success);
    assert!(stdout.contains("Production usage:"));
    assert!(stdout.contains("help prod"));
}

#[test]
fn test_interactive_unknown_command_then_exit() {
    let (success, stdout, stderr) = run_interactive_session(
        vec!["unknown-command".to_string(), "exit".to_string()],
        PORT_INTERACTIVE
    );
    println!("STDOUT:\n{stdout}\nSTDERR:\n{stderr}");

    assert!(success);
    assert!(stdout.contains("Commands:"));
}