use tokio::{
    io::{self, AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use sha2::{Digest, Sha256};
use std::path::Path;

use crate::{files::open_file_read, files_vault_errors::FileVaultError, protocole::Packet};

pub struct Customer {
    host: String,
    port: u64,
    stream: Option<TcpStream>,
    is_connected: bool,
}

impl Customer {
    pub async fn from(host: String, port: u64) -> Result<Self, FileVaultError> {
        let addres = format!("{host}:{port}");
        let connexion = Customer::connect_to(addres).await;
        let mut is_connected = false;

        let stream = match connexion {
            Ok(stream) => {
                is_connected = true;
                Some(stream)
            }
            Err(_) => None,
        };

        Ok(Customer {
            host,
            port,
            stream,
            is_connected,
        })
    }

    pub fn is_connected(&self) -> bool {
        self.is_connected
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

        stream.map_err(|error| FileVaultError::ConnectionError(error.to_string()))
    }

    pub async fn connexion(&mut self) -> Result<(), FileVaultError> {
        let connexion = Customer::connect_to(self.get_addres()).await;
        self.stream = match connexion {
            Ok(stream) => {
                self.is_connected = true;
                Some(stream)
            }
            Err(error) => return Err(error),
        };

        Ok(())
    }

    async fn print_progress(data_size: u64, size_progress: usize) {
        let size_bare = 20;
        let percent = ((size_progress as f64) / data_size as f64) * 100.0;

        let filled_blocs = ((size_progress as f64 / data_size as f64) * size_bare as f64) as usize;
        let blocs_vides = size_bare - filled_blocs;

        let filling = "*".repeat(filled_blocs);
        let empty = "_".repeat(blocs_vides);

        print!("\r[{}{}] {:>3.0}%", filling, empty, percent);

        io::stdout().flush().await.unwrap();
    }

    pub async fn send_file(
        &mut self,
        filename: String,
        root: Option<String>,
    ) -> Result<(), FileVaultError> {
        if !self.is_connected {
            self.connexion().await?;
        }

        let formated_filename = if !filename.contains("/") {
            filename.clone()
        } else {
            filename
                .split("/")
                .filter(|x| !x.is_empty())
                .last()
                .unwrap()
                .to_string()
        };

        let file_path = match root {
            Some(root) => Path::new(&root)
                .join(filename)
                .to_string_lossy()
                .into_owned(),
            None => filename,
        };
        let mut data = open_file_read(file_path)
            .await
            .map_err(|error| FileVaultError::FileOpeningError(error.to_string()))?;
        let data_size = data
            .metadata()
            .await
            .map_err(|e| FileVaultError::FileOpeningError(e.to_string()))?
            .len();

        let packet = Packet::from(formated_filename, data_size);

        let stream = self.stream.as_mut().ok_or_else(|| {
            FileVaultError::ConnectionError("TcpStream not connected".to_string())
        })?;

        if let Err(e) = stream.write_all(&packet.to_byte()).await {
            return Err(FileVaultError::TcpSendingError(e.to_string()));
        }

        let mut size_progress: usize = 0;

        let file_hash = {
            let mut buffer = [0_u8; 8_192];
            let mut hasher = Sha256::new();

            loop {
                let bytes_read = data
                    .read(&mut buffer)
                    .await
                    .map_err(|e| FileVaultError::FileOpeningError(e.to_string()))?;

                if bytes_read == 0 {
                    break;
                }

                size_progress += bytes_read;
                Customer::print_progress(data_size, size_progress).await;

                stream
                    .write_all(&buffer[..bytes_read])
                    .await
                    .map_err(|e| FileVaultError::TcpSendingError(e.to_string()))?;
                hasher.update(&buffer[..bytes_read]);
            }
            println!();
            hasher.finalize()
        };

        stream
            .write_all(&file_hash)
            .await
            .map_err(|e| FileVaultError::TcpSendingError(e.to_string()))?;

        println!("[CLIENT]: file succefull sent!");

        Ok(())
    }
}
