use std::{path::PathBuf, time::Duration};

use file_vault::{
    customer::Customer,
    files::{open_file_read, open_file_write, write_in_file},
    server::Server,
};
use tempfile::tempdir;
use tokio::{
    fs,
    io::AsyncReadExt,
    net::TcpListener,
    time::{sleep, Instant},
};

const HOST: &str = "::1";
const PORT_CREATE: u64 = 8070;
const FILE_TO_SAVE: &str = "test_server.txt";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(2);
const FILE_READY_TIMEOUT: Duration = Duration::from_secs(5);
const FILE_READY_POLL: Duration = Duration::from_millis(25);
const CONNECT_RETRIES: usize = 60;
const CONNECT_RETRY_DELAY: Duration = Duration::from_millis(50);

async fn wait_until_file_readable(path: &str, timeout: Duration, poll: Duration) -> bool {
    let deadline = Instant::now() + timeout;

    while Instant::now() < deadline {
        if open_file_read(path.to_string()).await.is_ok() {
            return true;
        }
        sleep(poll).await;
    }

    false
}

async fn create_and_fill_file(filename: String) {
    let mut file = open_file_write(&filename, false)
        .await
        .expect("unable to create test file");
    write_in_file(&mut file, b"test".to_vec())
        .await
        .expect("unable to write test file");
}

async fn connect_customer_with_retry(port: u64) -> Customer {
    for attempt in 0..CONNECT_RETRIES {
        let customer = Customer::from(HOST.to_string(), port)
            .await
            .expect("failed to create customer");
        if customer.is_connected() {
            return customer;
        }

        if attempt + 1 < CONNECT_RETRIES {
            sleep(CONNECT_RETRY_DELAY).await;
        }
    }

    panic!("unable to connect to server on {HOST}:{port}");
}

async fn send_file(filename: String, port: u64) {
    let mut customer = connect_customer_with_retry(port).await;
    create_and_fill_file(filename.clone()).await;

    assert!(open_file_read(filename.clone()).await.is_ok());
    assert!(customer.send_file(filename.clone(), None).await.is_ok());
    let _ = fs::remove_file(filename).await;
}

async fn receive_packet(port: u64, storage_root: PathBuf, log_root: PathBuf) {
    let filename = storage_root
        .join(FILE_TO_SAVE)
        .to_string_lossy()
        .into_owned();
    let history = log_root.join("history.log").to_string_lossy().into_owned();
    let mut server = Server::from_with_paths(
        HOST.to_string(),
        port,
        storage_root,
        PathBuf::from(&history),
    );

    assert_eq!(0, server.get_number_of_connexion().await);
    assert!(server.listening_async(DEFAULT_TIMEOUT).await.is_ok());
    assert_eq!(1, server.get_number_of_connexion().await);

    assert!(wait_until_file_readable(&filename, FILE_READY_TIMEOUT, FILE_READY_POLL).await);
    let mut received_file = String::new();
    open_file_read(filename.clone())
        .await
        .expect("unable to open received file")
        .read_to_string(&mut received_file)
        .await
        .expect("unable to read received file");
    assert_eq!("test", received_file);

    assert!(wait_until_file_readable(&history, FILE_READY_TIMEOUT, FILE_READY_POLL).await);
    let mut history_content = String::new();
    open_file_read(history.clone())
        .await
        .expect("unable to open history file")
        .read_to_string(&mut history_content)
        .await
        .expect("unable to read history file");
    assert!(history_content.contains("-  test_server.txt 4 - bytes"));

    let _ = fs::remove_file(filename).await;
    let _ = fs::remove_file(history).await;
}

#[tokio::test]
async fn test_create_server() {
    let server = Server::from(HOST.to_string(), PORT_CREATE);

    assert_eq!(format!("{HOST}:{PORT_CREATE}"), server.get_adress());
    assert_eq!(0, server.get_number_of_connexion().await);
}

#[tokio::test]
async fn test_server_listening() {
    let port = find_available_port().await;
    let temp_root = tempdir().expect("unable to create temp root");
    let source_root = temp_root.path().join("source");
    let storage_root = temp_root.path().join("storage");
    let log_root = temp_root.path().join("logs");
    fs::create_dir_all(&source_root)
        .await
        .expect("unable to create source directory");
    fs::create_dir_all(&storage_root)
        .await
        .expect("unable to create storage directory");
    fs::create_dir_all(&log_root)
        .await
        .expect("unable to create log directory");

    let filename = source_root
        .join(FILE_TO_SAVE)
        .to_string_lossy()
        .into_owned();
    let server_task = tokio::spawn(receive_packet(port, storage_root, log_root));

    send_file(filename, port).await;
    server_task.await.expect("server task panicked");
}

async fn find_available_port() -> u64 {
    let listener = TcpListener::bind((HOST, 0))
        .await
        .expect("unable to bind to ephemeral port");
    u64::from(
        listener
            .local_addr()
            .expect("unable to read local address")
            .port(),
    )
}
