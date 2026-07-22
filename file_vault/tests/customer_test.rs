use std::{
    fs,
    io::{BufReader, Read},
    net::TcpListener,
    path::Path,
    process,
    sync::atomic::{AtomicU64, Ordering},
    thread::sleep,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use file_vault::{
    customer::Customer,
    files::{open_file_read, open_file_write, write_in_file},
    protocole::Packet,
};

static FILE_COUNTER: AtomicU64 = AtomicU64::new(0);
const HOST: &str = "::1";
const PORT_WITHOUT_CONNEXION: u64 = 8090u64;
const PORT_WITH_CONNEXION: u64 = 8091u64;
const PORT_CONNEXION_SUCESS: u64 = 8092u64;
const PORT_CONNEXION_FAILS: u64 = 8093u64;
const PORT_SEND_FAILS: u64 = 8094u64;
const PORT_SEND_SUCCESS: u64 = 8095u64;
const PORT_SEND_SUCCESS_ROOT: u64 = 8096u64;

fn start_server(address: &str) {
    let get_listener = TcpListener::bind(address);
    assert!(get_listener.is_ok());

    let listener = get_listener.unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(_) => {
                break;
            }
            Err(_) => { /* connection failed */ }
        }
    }
}

fn start_server_for_send_success(address: &str) {
    let get_listener = TcpListener::bind(address);
    assert!(get_listener.is_ok());

    let listener = get_listener.unwrap();
    let get_stream = listener.accept();
    assert!(get_stream.is_ok());

    let (stream, _) = get_stream.unwrap();
    let mut reader = BufReader::new(stream);

    let packet = Packet::from_bytes(&mut reader);
    assert!(packet.is_ok());
    let packet = packet.unwrap();

    let mut payload = vec![0u8; packet.data_size() as usize];
    assert!(reader.read_exact(&mut payload).is_ok());

    let mut hash = [0u8; 32];
    assert!(reader.read_exact(&mut hash).is_ok());
}

fn get_file_name() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let idx = FILE_COUNTER.fetch_add(1, Ordering::Relaxed);

    Path::new("tests")
        .join(format!("test_{}_{}_{}.txt", process::id(), now, idx))
        .to_string_lossy()
        .into_owned()
}

fn get_file_name_brute() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let idx = FILE_COUNTER.fetch_add(1, Ordering::Relaxed);

    format!("test_{}_{}_{}.txt", process::id(), now, idx)
}

fn get_path(filename: String, root: String) -> String {
    Path::new(&root)
        .join(filename)
        .to_string_lossy()
        .into_owned()
}

fn create_and_fill_file(filename: String) {

    let get_file = open_file_write(filename.as_str(), false);
    assert!(get_file.is_ok());

    let mut file = get_file.unwrap();
    let bytes: Vec<u8> = "test".bytes().collect();

    assert!(write_in_file(&mut file, bytes.clone()).is_ok());
    drop(file);
}

#[test]
fn test_create_customer_without_connexion() {
    let get_customer = Customer::from(HOST.to_string(), PORT_WITHOUT_CONNEXION);
    assert!(get_customer.is_ok());

    let customer = get_customer.unwrap();

    assert!(!customer.is_connected());
    assert_eq!(format!("{HOST}:{PORT_WITHOUT_CONNEXION}"), customer.get_addres());
    assert_eq!(HOST, customer.get_host());
    assert_eq!(PORT_WITHOUT_CONNEXION, customer.get_port())
}

#[test]
fn test_create_customer_with_connexion() {
    let address = format!("{HOST}:{PORT_WITH_CONNEXION}");
    let server_address = address.clone();
    let thread = std::thread::spawn(move || {
        start_server(&server_address);
    });

    sleep(Duration::from_millis(100));

    let get_customer = Customer::from(HOST.to_string(), PORT_WITH_CONNEXION);
    assert!(get_customer.is_ok());

    let customer = get_customer.unwrap();

    assert!(customer.is_connected());
    assert_eq!(address, customer.get_addres());

    let _ = thread.join().unwrap();
}

#[test]
fn test_custmer_connexion_sucess() {
    let get_customer = Customer::from(HOST.to_string(), PORT_CONNEXION_SUCESS);
    assert!(get_customer.is_ok());

    let mut customer = get_customer.unwrap();

    assert!(!customer.is_connected());
    assert_eq!(format!("{HOST}:{PORT_CONNEXION_SUCESS}"), customer.get_addres());

    let address = format!("{HOST}:{PORT_CONNEXION_SUCESS}");
    let server_address = address.clone();
    let thread = std::thread::spawn(move || {
        start_server(&server_address);
    });

    sleep(Duration::from_millis(100));

    assert!(customer.connexion().is_ok());
    assert!(customer.is_connected());

    let _ = thread.join().unwrap();
}

#[test]
fn test_custmer_connexion_fails() {
    let get_customer = Customer::from(HOST.to_string(), PORT_CONNEXION_FAILS);
    assert!(get_customer.is_ok());

    let mut customer = get_customer.unwrap();

    assert!(!customer.is_connected());
    assert_eq!(format!("{HOST}:{PORT_CONNEXION_FAILS}"), customer.get_addres());

    assert!(customer.connexion().is_err());
    assert!(!customer.is_connected());
}

#[test]
fn test_send_packet_fails_for_connexion() {
    let get_customer = Customer::from(HOST.to_string(), PORT_SEND_FAILS);
    assert!(get_customer.is_ok());

    let mut customer = get_customer.unwrap();

    let filename = get_file_name();

    create_and_fill_file(filename.clone());

    let get_file = open_file_read(filename.clone());
    assert!(get_file.is_ok());
    

    let get_sent = customer.send_file(filename.clone(), None);
    let _ = fs::remove_file(filename);
    
    assert!(get_sent.is_err());

    assert!(get_sent.unwrap_err().to_string().contains("TcpStream connection Error:"));
}

#[test]
fn test_send_packet_success() {
    let address = format!("{HOST}:{PORT_SEND_SUCCESS}");
    let server_address = address.clone();
    let thread = std::thread::spawn(move || {
        start_server_for_send_success(&server_address);
    });

    sleep(Duration::from_millis(100));

    let get_customer = Customer::from(HOST.to_string(), PORT_SEND_SUCCESS);
    assert!(get_customer.is_ok());

    let mut customer = get_customer.unwrap();
    assert!(customer.is_connected());

    let filename = get_file_name();

    create_and_fill_file(filename.clone());

    let get_file = open_file_read(filename.clone());
    assert!(get_file.is_ok());
    

    let get_sent = customer.send_file(filename.clone(), None);
    let _ = fs::remove_file(filename);
    assert!(get_sent.is_ok());

    let _ = thread.join().unwrap();

}

#[test]
fn test_send_packet_with_root_sucess() {
    let address = format!("{HOST}:{PORT_SEND_SUCCESS_ROOT}");
    let server_address = address.clone();
    let thread = std::thread::spawn(move || {
        start_server_for_send_success(&server_address);
    });

    sleep(Duration::from_millis(100));

    let get_customer = Customer::from(HOST.to_string(), PORT_SEND_SUCCESS_ROOT);
    assert!(get_customer.is_ok());

    let mut customer = get_customer.unwrap();
    assert!(customer.is_connected());

    let filename = get_file_name_brute();
    let file_path = get_path(filename.clone(), "tests".to_string());

    create_and_fill_file(file_path.clone());

    let get_file = open_file_read(file_path.clone());
    assert!(get_file.is_ok());
    

    let get_sent = customer.send_file(filename.clone(), Some("tests".to_string()));
    let _ = fs::remove_file(file_path.clone());
    assert!(get_sent.is_ok());

    let _ = thread.join().unwrap();
}