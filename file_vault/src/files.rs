use std::fs;
use std::io::Write;

pub fn open_file_read(filename: String) -> std::io::Result<fs::File> {
    fs::OpenOptions::new().read(true).open(filename)
}

pub fn open_file_write(filename: &str, exist: bool) -> std::io::Result<fs::File> {
    fs::OpenOptions::new()
        .create(!exist)
        .truncate(true)
        .write(true)
        .open(filename)
}

pub fn write_in_file(file: &mut fs::File, payload: Vec<u8>) -> std::io::Result<()> {
    file.write_all(&payload)?;
    file.flush()?;
    Ok(())
}

pub fn open_file_append(filename: &str, exist: bool) -> std::io::Result<fs::File> {
    fs::OpenOptions::new()
        .create(!exist)
        .append(true)
        .open(filename)
}

pub fn add_line_in_file(file: &mut fs::File, payload: Vec<u8>) -> std::io::Result<()> {
    file.write_all(&payload)?;
    file.write_all(b"\n")?;
    file.flush()?;
    Ok(())
}