/* 
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

use std::{
    fs,
    io::Read,
    path::PathBuf,
    thread::{self, sleep},
    time::Duration
};

use file_vault::cli_helpers::{RuntimeMode, runtime_directories};
use file_vault::{
    customer::Customer,
    server::Server,
    files::{add_line_in_file, open_file_append, open_file_read, open_file_write, write_in_file}
};

const HOST: &str = "::1";
const PORT_CREATE: u64 = 8070u64;
const PORT_LISTENING: u64 = 8071u64;
const FILE_TO_SAVE: &str = "test_server.txt";
const DEFAULT_TIMEOUT: Duration = Duration::from_millis(500);
const FILE_READY_TIMEOUT: Duration = Duration::from_secs(2);
const FILE_READY_POLL: Duration = Duration::from_millis(25);

const PERFORMANCE : Mutex<HashMap<String, Vec<u64>>> = Mutex::new(HashMap::new());

struct Performance {
    table: Mutex<HashMap<String, Vec<u64>>>,
    filename: String,
    size: u32
}

impl Performance {
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

    fn send_file(filename: String) -> bool {
        let get_customer = Customer::from(HOST.to_string(), PORT_LISTENING);
        let mut customer = get_customer.unwrap();

        if open_file_read(filename.clone()).is_err() {
            return false;
        };
        
        if customer.send_file(filename.clone(), None).is_err() {
            return false;
        };
        let _ = fs::remove_file(filename);

        true
    }
}

fn receive_packet(filename: String, size: u32) {

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

    let start_time: u64 = Instant::now().elapsed().as_millis() as u64;
    

    let mut server = Server::from_with_paths(
        HOST.to_string(),
        PORT_LISTENING,
        PathBuf::from(storage_root),
        PathBuf::from(history.clone()),
    );

    let has_listen = server.listening(DEFAULT_TIMEOUT);

    let runtime: u64 = Instant::now().elapsed().as_millis() as u64 - start_time;


    
    let _ = fs::remove_file(filename);

    let _ = fs::remove_file(history);
}

fn run_one_time() {
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
}*/