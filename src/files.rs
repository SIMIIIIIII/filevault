use tokio::fs;
use tokio::io::AsyncWriteExt;

pub async fn open_file_read(filename: String) -> std::io::Result<fs::File> {
    fs::OpenOptions::new().read(true).open(filename).await
}

pub async fn open_file_write(filename: &str, exist: bool) -> std::io::Result<fs::File> {
    fs::OpenOptions::new()
        .create(!exist)
        .truncate(true)
        .write(true)
        .open(filename).await
}

pub async fn write_in_file(file: &mut fs::File, payload: Vec<u8>) -> std::io::Result<()> {
    file.write_all(&payload).await?;
    file.flush().await?;
    Ok(())
}

pub async fn open_file_append(filename: &str, exist: bool) -> std::io::Result<fs::File> {
    fs::OpenOptions::new()
        .create(!exist)
        .append(true)
        .open(filename).await
}

pub async fn add_line_in_file(file: &mut fs::File, payload: Vec<u8>) -> std::io::Result<()> {
    file.write_all(&payload).await?;
    file.write_all(b"\n").await?;
    file.flush().await?;
    Ok(())
}