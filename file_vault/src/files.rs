use std::fs;
use std::io::Write;

pub fn open_file_read(filename: &str) -> std::io::Result<fs::File> {
    fs::OpenOptions::new().read(true).open(filename)
}

pub fn open_file_write(filename: &str) -> std::io::Result<fs::File> {
    fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(filename)
}

pub fn write_in_file(file: &mut fs::File, payload: Vec<u8>) -> std::io::Result<()> {
    file.write_all(&payload)?;
    file.flush()?;
    Ok(())
}