use std::{
    fs::{self, File},
    io::Read,
    path::Path,
    process,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use file_vault::files::{self, open_file_write, write_in_file};

static FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

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


#[test]
fn test_open_file_fails_for_non_existing_file() {
    let file = files::open_file_read("test.txt".to_string());
    
    assert!(file.is_err());
}

#[test]
fn test_open_file_succes() {
    let filename = get_file_name();

    assert!(File::create(filename.clone()).is_ok());
    assert!(files::open_file_read(filename.clone()).is_ok());

    let _ = fs::remove_file(filename);
}

#[test]
fn test_open_file_write() {
    let filename = get_file_name();

    assert!(open_file_write(filename.as_str()).is_ok());
    assert!(Path::new(&filename).exists());
    let _ = fs::remove_file(filename);
}

#[test]
fn test_write_in_file() {
    let filename = get_file_name();

    let get_file = open_file_write(filename.as_str());
    assert!(get_file.is_ok());

    let mut file = get_file.unwrap();
    let bytes: Vec<u8> = "test".bytes().collect();

    assert!(write_in_file(&mut file, bytes.clone()).is_ok());
    drop(file);

    let reopened = files::open_file_read(filename.clone());
    assert!(reopened.is_ok());
    
    let mut res: Vec<u8> = Vec::new();
    assert!(reopened.unwrap().read_to_end(&mut res).is_ok());

    assert_eq!(res.clone(), bytes.clone());
    let _ = fs::remove_file(filename);
}