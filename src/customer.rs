use tokio::{
    io::{AsyncReadExt, AsyncWriteExt}, net::TcpStream,
};

use std::path::Path;
use sha2::{Digest, Sha256};

use crate::{
    files::open_file_read,
    files_vault_errors::FileVaultError,
    protocole::Packet
};

pub struct Customer {
    host: String,
    port: u64,
    stream : Option<TcpStream>,
    is_connected: bool,
}

impl Customer {
    pub async fn from(host: String, port: u64) -> Result<Self, FileVaultError> {
        let addres = format!("{host}:{port}");
        let connexion = Customer::connect_to(addres).await;
        let mut is_connected = false;

        let stream = if connexion.is_err() {
            None
        } else {
            is_connected = true;
            Some(connexion.unwrap())
        };

        Ok(Customer {
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

    pub fn get_host(&self) -> String {
        self.host.clone()
    }

    pub fn get_port(&self) -> u64 {
        self.port
    }

    pub async fn connect_to(addres: String) -> Result<TcpStream, FileVaultError> {
        let stream = TcpStream::connect(addres).await;

        if stream.is_err() {
            return Err(FileVaultError::ConnectionError(stream.unwrap_err().to_string()));
        }

        Ok(stream.unwrap())
    }

    pub async fn connexion(&mut self) -> Result<(), FileVaultError> {
        let connexion = Customer::connect_to(self.get_addres()).await;
        self.stream = if connexion.is_err() {
            return Err(connexion.unwrap_err());
        } else {
            self.is_connected = true;
            Some(connexion.unwrap())
        };
        
        Ok(())
    }

    pub async fn send_file(&mut self, filename: String, root: Option<String>) -> Result<(), FileVaultError> {
        if !self.is_connected {
            self.connexion().await?;
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
        ).await;

        if file.is_err() {
            return Err(FileVaultError::FileOpeningError(file.unwrap_err().to_string()));
        }

        let mut data = file.unwrap();
        let data_size = data
            .metadata()
            .await
            .map_err(|e| FileVaultError::FileOpeningError(e.to_string()))?
            .len();

        let packet = Packet::from(formated_filename, data_size);

        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| FileVaultError::ConnectionError("TcpStream not connected".to_string()))?;


        if let Err(e) = stream.write_all(&packet.to_byte()).await {
            return Err(FileVaultError::TcpSendingError(e.to_string()));
        }

        let file_hash = {
            let mut buffer = [0_u8; 8_192];
            let mut hasher = Sha256::new();

            loop {
                let bytes_read = data.read(&mut buffer).await
                .map_err(|e| FileVaultError::FileOpeningError(e.to_string()))?;

                if bytes_read == 0 {
                    break;
                }

                stream.write_all(&buffer[..bytes_read]).await
                .map_err(|e| FileVaultError::TcpSendingError(e.to_string()))?;
                hasher.update(&buffer[..bytes_read]);
                
            }
            
            hasher.finalize()
        };

        stream
            .write_all(&file_hash).await
            .map_err(|e| FileVaultError::TcpSendingError(e.to_string()))?;

        println!("[CLIENT]: file succefull sent!");

        Ok(())

    }
}