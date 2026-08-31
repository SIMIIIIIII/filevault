use std::{
    path::Path,
    process,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use tokio::{
    fs::{self, File},
    io::AsyncReadExt,
};

use file_vault::files::{
    self,
    add_line_in_file,
    open_file_append,
    open_file_write,
    write_in_file,
};

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


#[tokio::test]
async fn test_open_file_fails_for_non_existing_file() {
    let file = files::open_file_read("test.txt".to_string()).await;
    
    assert!(file.is_err());
}

#[tokio::test]
async fn test_open_file_succes() {
    let filename = get_file_name();

    assert!(File::create(filename.clone()).await.is_ok());
    assert!(files::open_file_read(filename.clone()).await.is_ok());

    let _ = fs::remove_file(filename).await;
}

#[tokio::test]
async fn test_open_file_write() {
    let filename = get_file_name();

    assert!(open_file_write(filename.as_str(), false).await.is_ok());
    assert!(Path::new(&filename).exists());
    let _ = fs::remove_file(filename).await;
}

#[tokio::test]
async fn test_write_in_file() {
    let filename = get_file_name();

    let get_file = open_file_write(filename.as_str(), false).await;
    assert!(get_file.is_ok());

    let mut file = get_file.unwrap();
    let bytes: Vec<u8> = "test".bytes().collect();

    assert!(write_in_file(&mut file, bytes.clone()).await.is_ok());
    drop(file);

    let reopened = files::open_file_read(filename.clone()).await;
    assert!(reopened.is_ok());
    
    let mut res: Vec<u8> = Vec::new();
    assert!(reopened.unwrap().read_to_end(&mut res).await.is_ok());

    assert_eq!(res.clone(), bytes.clone());
    let _ = fs::remove_file(filename).await;
}

#[tokio::test]
async fn test_open_file_append() {
    let filename = get_file_name();

    let get_file = open_file_write(filename.as_str(), false).await;
    assert!(get_file.is_ok());

    let mut file = get_file.unwrap();
    assert!(write_in_file(&mut file, b"old".to_vec()).await.is_ok());
    drop(file);

    let append_file = open_file_append(filename.as_str(), true).await;
    assert!(append_file.is_ok());

    let mut file = append_file.unwrap();
    assert!(write_in_file(&mut file, b"new".to_vec()).await.is_ok());
    drop(file);

    let reopened = files::open_file_read(filename.clone()).await;
    assert!(reopened.is_ok());

    let mut content = Vec::new();
    assert!(reopened.unwrap().read_to_end(&mut content).await.is_ok());

    assert_eq!(content, b"oldnew".to_vec());
    let _ = fs::remove_file(filename).await;
}

#[tokio::test]
async fn test_add_line_in_file() {
    let filename = get_file_name();

    let get_file = open_file_append(filename.as_str(), false).await;
    assert!(get_file.is_ok());

    let mut file = get_file.unwrap();
    assert!(add_line_in_file(&mut file, b"line1".to_vec()).await.is_ok());
    assert!(add_line_in_file(&mut file, b"line2".to_vec()).await.is_ok());
    drop(file);

    let reopened = files::open_file_read(filename.clone()).await;
    assert!(reopened.is_ok());

    let mut content = String::new();
    assert!(reopened.unwrap().read_to_string(&mut content).await.is_ok());

    assert_eq!(content, "line1\nline2\n");
    let _ = fs::remove_file(filename).await;
}