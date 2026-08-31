use tokio::{
    fs,
    io::{AsyncReadExt},
    net::TcpListener,
    time::{Duration, sleep},
};

use std::{
    process,
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH}
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

async fn start_server(address: &str) {
    let listener = TcpListener::bind(address)
        .await
        .expect("unable to bind listener");

    loop {
        match listener.accept().await {
            Ok((_stream, _address)) => {
                break;
            }
            Err(_) => {
            }
        }
    }
}

async fn start_server_for_send_success(address: &str) {
    let listener = TcpListener::bind(address).await.unwrap();
    let (stream, _) = listener.accept().await.unwrap();
    let mut reader = tokio::io::BufReader::new(stream);

    let packet = Packet::from_bytes(&mut reader).await.unwrap();

    let mut payload = vec![0_u8; packet.data_size() as usize];
    AsyncReadExt::read_exact(&mut reader, &mut payload)
        .await
        .unwrap();

    let mut hash = [0_u8; 32];
    AsyncReadExt::read_exact(&mut reader, &mut hash)
        .await
        .unwrap();
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

async fn create_and_fill_file(filename: String) {

    let get_file = open_file_write(filename.as_str(), false).await;
    assert!(get_file.is_ok());

    let mut file = get_file.unwrap();
    let bytes: Vec<u8> = "test".bytes().collect();

    assert!(write_in_file(&mut file, bytes.clone()).await.is_ok());
    drop(file);
}

#[tokio::test]
async fn test_create_customer_without_connexion() {
    let get_customer = Customer::from(HOST.to_string(), PORT_WITHOUT_CONNEXION).await;
    assert!(get_customer.is_ok());

    let customer = get_customer.unwrap();

    assert!(!customer.is_connected());
    assert_eq!(format!("{HOST}:{PORT_WITHOUT_CONNEXION}"), customer.get_addres());
    assert_eq!(HOST, customer.get_host());
    assert_eq!(PORT_WITHOUT_CONNEXION, customer.get_port())
}

#[tokio::test]
async fn test_create_customer_with_connexion() {
    let address = format!("{HOST}:{PORT_WITH_CONNEXION}");
    let server_address = address.clone();
    let thread = tokio::spawn(async move {
        start_server(&server_address).await;
    });

    sleep(Duration::from_millis(100)).await;

    let get_customer = Customer::from(HOST.to_string(), PORT_WITH_CONNEXION).await;
    assert!(get_customer.is_ok());

    let customer = get_customer.unwrap();

    assert!(customer.is_connected());
    assert_eq!(address, customer.get_addres());

    let _ = thread.await.expect("server task panicked");
}

#[tokio::test]
async fn test_custmer_connexion_sucess() {
    let get_customer = Customer::from(HOST.to_string(), PORT_CONNEXION_SUCESS).await;
    assert!(get_customer.is_ok());

    let mut customer = get_customer.unwrap();

    assert!(!customer.is_connected());
    assert_eq!(format!("{HOST}:{PORT_CONNEXION_SUCESS}"), customer.get_addres());

    let address = format!("{HOST}:{PORT_CONNEXION_SUCESS}");
    let server_address = address.clone();
    let thread = tokio::spawn(async move {
        start_server(&server_address).await;
    });

    sleep(Duration::from_millis(100)).await;

    assert!(customer.connexion().await.is_ok());
    assert!(customer.is_connected());

    let _ = thread.await.expect("server task panicked");
}

#[tokio::test]
async fn test_custmer_connexion_fails() {
    let get_customer = Customer::from(HOST.to_string(), PORT_CONNEXION_FAILS).await;
    assert!(get_customer.is_ok());

    let mut customer = get_customer.unwrap();

    assert!(!customer.is_connected());
    assert_eq!(format!("{HOST}:{PORT_CONNEXION_FAILS}"), customer.get_addres());

    assert!(customer.connexion().await.is_err());
    assert!(!customer.is_connected());
}

#[tokio::test]
async fn test_send_packet_fails_for_connexion() {
    let get_customer = Customer::from(HOST.to_string(), PORT_SEND_FAILS).await;
    assert!(get_customer.is_ok());

    let mut customer = get_customer.unwrap();

    let filename = get_file_name();

    create_and_fill_file(filename.clone()).await;

    let get_file = open_file_read(filename.clone()).await;
    assert!(get_file.is_ok());
    

    let get_sent = customer.send_file(filename.clone(), None).await;
    let _ = fs::remove_file(filename).await;
    
    assert!(get_sent.is_err());

    assert!(get_sent.unwrap_err().to_string().contains("TcpStream connection Error:"));
}

#[tokio::test]
async fn test_send_packet_success() {
    let address = format!("{HOST}:{PORT_SEND_SUCCESS}");
    let server_address = address.clone();
    let thread = tokio::spawn(async move {
        start_server_for_send_success(&server_address).await;
    });

    sleep(Duration::from_millis(100)).await;

    let get_customer = Customer::from(HOST.to_string(), PORT_SEND_SUCCESS).await;
    assert!(get_customer.is_ok());

    let mut customer = get_customer.unwrap();
    assert!(customer.is_connected());

    let filename = get_file_name();

    create_and_fill_file(filename.clone()).await;

    let get_file = open_file_read(filename.clone()).await;
    assert!(get_file.is_ok());
    

    let get_sent = customer.send_file(filename.clone(), None).await;
    let _ = fs::remove_file(filename).await;
    assert!(get_sent.is_ok());

    let _ = thread.await.expect("server task panicked");

}

#[tokio::test]
async fn test_send_packet_with_root_sucess() {
    let address = format!("{HOST}:{PORT_SEND_SUCCESS_ROOT}");
    let server_address = address.clone();
    let thread = tokio::spawn(async move {
        start_server_for_send_success(&server_address).await;
    });

    sleep(Duration::from_millis(100)).await;

    let get_customer = Customer::from(HOST.to_string(), PORT_SEND_SUCCESS_ROOT).await;
    assert!(get_customer.is_ok());

    let mut customer = get_customer.unwrap();
    assert!(customer.is_connected());

    let filename = get_file_name_brute();
    let file_path = get_path(filename.clone(), "tests".to_string());

    create_and_fill_file(file_path.clone()).await;

    let get_file = open_file_read(file_path.clone()).await;
    assert!(get_file.is_ok());
    

    let get_sent = customer.send_file(filename.clone(), Some("tests".to_string())).await;
    let _ = fs::remove_file(file_path.clone()).await;
    assert!(get_sent.is_ok());

    let _ = thread.await.expect("server task panicked");
}