use std::{
    io::Write,
    net::TcpStream,
    path::Path
};

use crate::{
    files::open_file_read,
    files_vault_errors::FileVaultError,
    hashing::HashingWriter,
    protocole::Packet
};

pub struct Custormer {
    host: String,
    port: u64,
    stream : Option<TcpStream>,
    is_connected: bool,
}

impl Custormer {
    pub fn from(host: String, port: u64) -> Result<Self, FileVaultError> {
        let addres = format!("{host}:{port}");
        let connexion = Custormer::connect_to(addres);
        let mut is_connected = false;

        let stream = if connexion.is_err() {
            None
        } else {
            is_connected = true;
            Some(connexion.unwrap())
        };

        Ok(Custormer {
            host: host,
            port: port,
            stream: stream,
            is_connected: is_connected,
        })
    }

    pub fn is_connected(&self) -> bool {
        self.is_connected.clone()
    }

    pub fn get_addres(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub fn connect_to(addres: String) -> Result<TcpStream, FileVaultError> {
        let stream = TcpStream::connect(addres);

        if stream.is_err() {
            return Err(FileVaultError::ConnectionError(stream.unwrap_err().to_string()));
        }

        Ok(stream.unwrap())
    }

    pub fn connexion(&mut self) -> Result<(), FileVaultError> {
        let connexion = Custormer::connect_to(self.get_addres());
        self.stream = if connexion.is_err() {
            return Err(connexion.unwrap_err());
        } else {
            self.is_connected = true;
            Some(connexion.unwrap())
        };
        
        Ok(())
    }

    pub fn send_file(&mut self, filename: String, root: Option<String>) -> Result<(), FileVaultError> {
        if !self.is_connected {
            self.connexion()?;
        }

        let formated_filename = if !filename.contains("/") {
            filename.clone()
        } else {
            filename.split("/").filter(|x| !x.is_empty()).last().unwrap().to_string()
        };

        let file = open_file_read(
            if root.is_none() {
                filename
            } else {
                Path::new(&root.unwrap())
                    .join(filename)
                    .to_string_lossy()
                    .into_owned()
            }
        );

        if file.is_err() {
            return Err(FileVaultError::FileOpeningError(file.unwrap_err().to_string()));
        }

        let mut data = file.unwrap();
        let data_size = data
            .metadata()
            .map_err(|e| FileVaultError::FileOpeningError(e.to_string()))?
            .len();

        let packet = Packet::from(formated_filename, data_size);

        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| FileVaultError::ConnectionError("TcpStream not connected".to_string()))?;


        if let Err(e) = stream.write_all(&packet.to_byte()) {
            return Err(FileVaultError::TcpSendingError(e.to_string()));
        }

        let file_hash = {
            let mut hashing_writer = HashingWriter::new(stream);
            std::io::copy(&mut data, &mut hashing_writer)
                .map_err(|e| FileVaultError::TcpSendingError(e.to_string()))?;
            hashing_writer.finalize()
        };

        stream
            .write_all(&file_hash)
            .map_err(|e| FileVaultError::TcpSendingError(e.to_string()))?;

        println!("[CLIENT]: file succefull sent!");

        Ok(())

    }
}