use std::io::{Cursor};

use file_vault::{files_vault_errors::FileVaultError, protocole::{self, Packet}};

#[test]
fn test_create_packet() {
    let packet = protocole::Packet::from(
        "test1".to_string(),
        50
    );

    assert_eq!("test1".to_string(), packet.get_name());
    assert_eq!(50, packet.data_size());
}

#[test]
fn test_packet_to_byte() {
    let packet = protocole::Packet::from(
        "test1".to_string(),
        50
    );

    let received_bytes = packet.to_byte();
    let mut expected_bytes: Vec<u8> = Vec::new();

    expected_bytes.extend_from_slice(("test1".len() as u32).to_be_bytes().as_slice());
    expected_bytes.extend_from_slice("test1".as_bytes());
    expected_bytes.extend_from_slice((50u64).to_be_bytes().as_slice());

    assert_eq!(expected_bytes, received_bytes);

}

#[test]
fn test_bytes_to_packet_sucess() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(("test1".len() as u32).to_be_bytes().as_slice());
    bytes.extend_from_slice("test1".as_bytes());
    bytes.extend_from_slice((50u64).to_be_bytes().as_slice());

    let mut cursor = Cursor::new(bytes);
    let packet = Packet::from_bytes(&mut cursor).unwrap();

    assert_eq!("test1".to_string(), packet.get_name());
    assert_eq!(50, packet.data_size());
}

#[test]
fn test_bytes_to_packet_fails() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(("test1".len() as u16).to_be_bytes().as_slice());
    bytes.extend_from_slice("test1".as_bytes());
    bytes.extend_from_slice((50u64).to_be_bytes().as_slice());

    let mut cursor = Cursor::new(bytes);
    
    let get_packet = Packet::from_bytes(&mut cursor);

    assert!(get_packet.is_err());

    let error = get_packet.unwrap_err();
    assert_eq!(FileVaultError::PacketCorrupted, error);
}