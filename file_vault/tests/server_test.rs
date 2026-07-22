use std::{
    fs,
    io::Read,
    path::PathBuf,
    thread::{self, sleep},
    time::Duration
};

use file_vault::{
    cli_helpers::{ensure_runtime_directories, runtime_directories, RuntimeMode},
    customer::Custormer,
    files::{open_file_read, open_file_write, write_in_file},
    server::Server
};


const HOST: &str = "::1";
const PORT_CREATE: u64 = 8070u64;
const PORT_LISTENING: u64 = 8071u64;
const FILE_TO_SAVE: &str = "test_server.txt";
const DEFAULT_TIMEOUT: Duration = Duration::from_millis(500);
const FILE_READY_TIMEOUT: Duration = Duration::from_secs(2);
const FILE_READY_POLL: Duration = Duration::from_millis(25);

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

fn send_file(filename: String) {
    let get_customer = Custormer::from(HOST.to_string(), PORT_LISTENING);
    assert!(get_customer.is_ok());

    let mut customer = get_customer.unwrap();
    assert!(customer.is_connected());

    create_and_fill_file(filename.clone());

    let get_file = open_file_read(filename.clone());
    assert!(get_file.is_ok());
    

    let get_sent = customer.send_file(filename.clone(), None);
    let _ = fs::remove_file(filename);
    assert!(get_sent.is_ok()); 
}

fn receive_packet() {
    assert!(ensure_runtime_directories(RuntimeMode::Test).is_ok());

    let (storage_root, log_root) = runtime_directories(RuntimeMode::Test);

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
        PORT_LISTENING,
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
    let (storage_root, _) = runtime_directories(RuntimeMode::Test);

    let filename: String = storage_root
        .join(FILE_TO_SAVE)
        .to_string_lossy()
        .to_string();

    let thead = thread::spawn(move || {
        receive_packet();
    });
    sleep(Duration::from_millis(100));

    send_file(filename);

    let _ = thead.join().unwrap();
}