use std::{
    fs::{self},
    io::{BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    path::Path, sync::{LazyLock, Mutex},
};

use crate::{
    files::{open_file_write, write_in_file},
    files_vault_errors::FileVaultError,
    protocole::Packet
};

use chrono::Utc;
use sha2::{
    Digest,
    Sha256
};


pub struct Server {
    numbers_of_connexion : LazyLock<Mutex<u128>>,
    host: String,
    port: u64
}


impl Server {
    pub fn from(host: String, port: u64) -> Self {
        Server {
            numbers_of_connexion: LazyLock::new(|| Mutex::new(0u128)),
            host,
            port
        }
    }

    fn get_adress(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    fn manage_customer(stream: TcpStream) -> Result<(), FileVaultError> {
        let mut reader = BufReader::new(stream);

        let packet = Packet::from_bytes(&mut reader)?;

        let mut file = open_file_write(
            &Path::new("server_files")
                .join(packet.get_name().as_str())
                .to_string_lossy()
                .into_owned()
            )
            .map_err(|e| FileVaultError::FileOpeningError(e.to_string()))?;

        Server::write_data(&mut reader, &mut file, packet.data_size())?;

        let _ = Self::update_history(packet.get_name(), packet.data_size());

        Ok(())
    }

    fn add_new_connexion(&mut self) {
        let mut number_of_connexion = self.numbers_of_connexion.lock().unwrap();
        *number_of_connexion = number_of_connexion.saturating_add(1);
    }

    fn update_history(filename: String, size: u64) -> Result<(), FileVaultError> {
        let now = Utc::now().format("%Y-%m-%d %H:%M:%S");
        let entry = format!("[SERVER]: {now}  -  {filename} {size} \n");

        let mut file = open_file_write("history.log")
            .map_err(|e| FileVaultError::FileOpeningError(e.to_string()))?;

        write_in_file(&mut file, entry.bytes().collect())
            .map_err(|e| FileVaultError::FileWritingError(e.to_string()))?;

        Ok(())
    }

    pub fn write_data(
        reader: &mut BufReader<TcpStream>,
        file: &mut fs::File,
        data_size: u64,
    ) -> Result<(), FileVaultError> {
        let mut chunk = [0u8; 1024];
        let mut hasher = Sha256::new();
        let mut remaining = data_size;

        while remaining > 0 {
            let to_read = usize::min(remaining as usize, chunk.len());
            let read_bytes = reader
                .read(&mut chunk[..to_read])
                .map_err(|_| FileVaultError::MissingPayload)?;

            if read_bytes == 0 {
                return Err(FileVaultError::MissingPayload);
            }

            file.write_all(&chunk[..read_bytes])
                .map_err(|e| FileVaultError::FileWritingError(e.to_string()))?;
            hasher.update(&chunk[..read_bytes]);
            remaining -= read_bytes as u64;
        }

        let digest = hasher.finalize();
        let mut computed_hash = [0u8; 32];
        computed_hash.copy_from_slice(&digest);

        let mut received_hash = [0u8; 32];
        reader
            .read_exact(&mut received_hash)
            .map_err(|_| FileVaultError::MissingPayload)?;

        if received_hash != computed_hash {
            return Err(FileVaultError::HashMismatch);
        }

        file.flush()
            .map_err(|e| FileVaultError::FileWritingError(e.to_string()))?;
        Ok(())
    }

    pub fn listening(&mut self) -> Result<(), FileVaultError> {
        let listener = TcpListener::bind(self.get_adress())
            .map_err(|e| FileVaultError::ConnectionError(e.to_string()))?;

        println!("[SERVER]: Listening on port {}", self.port);

        for flux in listener.incoming() {
            match flux {
                Ok(stream) => {
                    self.add_new_connexion();
                    std::thread::spawn(|| Self::manage_customer(stream));
                },
                Err(e) => {
                    return Err(FileVaultError::ConnectionError(e.to_string()));
                }
            }
        }
        Ok(())
    }
}