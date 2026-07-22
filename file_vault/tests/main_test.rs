use std::{fs, process::Command};

const PORT_RUN_SUCCES: &str = "8010";
const PORT_RUN_FAILED: &str = "8011";

#[test]
fn test_run_customer_success() {
    let output = Command::new(env!("CARGO_BIN_EXE_file_vault"))
        .arg("--test")
        .arg("customer")
        .arg("--methode")
        .arg("POST")
        .arg("localhost")
        .arg(PORT_RUN_SUCCES)
        .arg("--filename")
        .arg("files.rs")
        .arg("--root")
        .arg("src")
        .output()
        .expect("failed to run binary");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid utf8");

    assert!(stdout.contains(format!("[SERVER]: Listening on port {PORT_RUN_SUCCES}").as_str()));
    assert!(stdout.contains("[CLIENT]: file succefull sent!"));
    

    

    let _ = fs::remove_dir_all("tests/server_files");
    let _ = fs::remove_dir_all("tests/log_files");
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

    //let stdout = String::from_utf8(output.stdout).expect("stdout should be valid utf8");

    //assert!(stdout.contains("[ERROR]: Usage:"));
    //assert_eq!("icic", stdout);

}

#[test]
fn test_get_help() {
    let output = Command::new(env!("CARGO_BIN_EXE_file_vault"))
        .arg("help")
        .output()
        .expect("failed to run binary");

    assert!(!output.status.success());

    //let stdout = String::from_utf8(output.stdout).expect("stdout should be valid utf8");

    //assert!(stdout.contains("[ERROR]: Usage:"));
}