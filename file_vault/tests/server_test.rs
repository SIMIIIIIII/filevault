use std::{
    fs,
    io::Read,
    net::TcpListener,
    path::PathBuf,
    thread::{self, sleep},
    time::Duration
};

use file_vault::{
    customer::Customer,
    files::{open_file_read, open_file_write, write_in_file},
    server::Server
};
use tempfile::tempdir;


const HOST: &str = "::1";
const PORT_CREATE: u64 = 8070u64;
const FILE_TO_SAVE: &str = "test_server.txt";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(2);
const FILE_READY_TIMEOUT: Duration = Duration::from_secs(5);
const FILE_READY_POLL: Duration = Duration::from_millis(25);
const CONNECT_RETRIES: usize = 60;
const CONNECT_RETRY_DELAY: Duration = Duration::from_millis(50);

fn wait_until_file_readable(path: &str, timeout: Duration, poll: Duration) -> bool {
    let start = std::time::Instant::now();

    while start.elapsed() < timeout {
        if open_file_read(path.to_string()).is_ok() {
            return true;
        }
        sleep(poll);
    }

    false
}


fn create_and_fill_file(filename: String) {
    let get_file = open_file_write(filename.as_str(), false);
    assert!(get_file.is_ok());

    let mut file = get_file.unwrap();
    let bytes: Vec<u8> = "test".bytes().collect();

    assert!(write_in_file(&mut file, bytes.clone()).is_ok());
    drop(file);
}

fn connect_customer_with_retry(port: u64) -> Customer {
    for attempt in 0..CONNECT_RETRIES {
        let customer = Customer::from(HOST.to_string(), port).expect("failed to create customer");
        if customer.is_connected() {
            return customer;
        }

        if attempt + 1 < CONNECT_RETRIES {
            sleep(CONNECT_RETRY_DELAY);
        }
    }

    panic!("unable to connect to server on {HOST}:{port}");
}

fn send_file(filename: String, port: u64) {
    let mut customer = connect_customer_with_retry(port);

    create_and_fill_file(filename.clone());

    let get_file = open_file_read(filename.clone());
    assert!(get_file.is_ok());
    

    let get_sent = customer.send_file(filename.clone(), None);
    let _ = fs::remove_file(filename);
    assert!(get_sent.is_ok()); 
}

fn receive_packet(port: u64, storage_root: PathBuf, log_root: PathBuf) {
    let filename: String = storage_root
        .join(FILE_TO_SAVE)
        .to_string_lossy()
        .to_string();

    let history: String = log_root
        .join("history.log")
        .to_string_lossy()
        .to_owned()
        .to_string();

    let mut server = Server::from_with_paths(
        HOST.to_string(),
        port,
        PathBuf::from(storage_root),
        PathBuf::from(history.clone()),
    );
    assert_eq!(0, server.get_number_of_connexion());

    let has_listen = server.listening(DEFAULT_TIMEOUT);

    assert!(has_listen.is_ok());
    
    assert_eq!(1, server.get_number_of_connexion());

    assert!(wait_until_file_readable(&filename, FILE_READY_TIMEOUT, FILE_READY_POLL));

    let get_sent_file = open_file_read(filename.clone());
    assert!(get_sent_file.is_ok());

    let mut sent_file : String = String::new();
    assert!(get_sent_file.unwrap().read_to_string(&mut sent_file).is_ok());

    assert!(!sent_file.is_empty());
    assert_eq!("test", sent_file);

    assert!(wait_until_file_readable(&history, FILE_READY_TIMEOUT, FILE_READY_POLL));

    let get_history = open_file_read(history.clone());
    assert!(get_history.is_ok());

    let mut history_content : String = String::new();
    assert!(get_history.unwrap().read_to_string(&mut history_content).is_ok());

    assert!(!history_content.is_empty());
    assert!(history_content.contains("-  test_server.txt 4 - bytes"));
    
    let _ = fs::remove_file(filename);

    let _ = fs::remove_file(history);
}

#[test]
fn test_create_server() {
    let server = Server::from(HOST.to_string(), PORT_CREATE);

    assert_eq!(format!("{}:{}", HOST, PORT_CREATE), server.get_adress());
    assert_eq!(0, server.get_number_of_connexion());
}

#[test]
fn test_server_listening() {
    let port = find_available_port();
    let temp_root = tempdir().expect("unable to create temp root");
    let storage_root = temp_root.path().join("storage");
    let log_root = temp_root.path().join("logs");
    assert!(fs::create_dir_all(&storage_root).is_ok());
    assert!(fs::create_dir_all(&log_root).is_ok());

    let filename: String = storage_root
        .join(FILE_TO_SAVE)
        .to_string_lossy()
        .to_string();

    let thead = thread::spawn(move || {
        receive_packet(port, storage_root, log_root);
    });

    send_file(filename, port);

    let _ = thead.join().unwrap();
}

fn find_available_port() -> u64 {
    let listener = TcpListener::bind((HOST, 0)).expect("unable to bind to ephemeral port");
    let port = listener
        .local_addr()
        .expect("unable to read local address")
        .port();
    drop(listener);
    u64::from(port)
}