use tokio::{
    net::{TcpListener,TcpStream},
    time::{Instant, sleep_until, Duration},
    fs,
    io::{AsyncWriteExt, BufReader, AsyncReadExt},
    sync::Mutex,
};

use std::{
    path::PathBuf,
};

use crate::{
    db::{connexion_db, insert_file}, files::{
        add_line_in_file,
        open_file_append,
        open_file_write
    }, files_vault_errors::FileVaultError, protocole::Packet,
};

use chrono::Utc;
use sha2::{Digest, Sha256};


pub struct Server {
    numbers_of_connexion: Mutex<u128>,
    host: String,
    port: u64,
    storage_root: PathBuf,
    history_path: PathBuf,
}

impl Server {
    pub fn from(host: String, port: u64) -> Self {
        Self {
            numbers_of_connexion: Mutex::new(0u128),
            host,
            port,
            storage_root: PathBuf::from("server_files"),
            history_path: PathBuf::from("log_files/history.log"),
        }
    }

    pub fn from_with_history_path(host: String, port: u64, history_path: PathBuf) -> Self {
        Self {
            numbers_of_connexion: Mutex::new(0u128),
            host,
            port,
            storage_root: PathBuf::from("server_files"),
            history_path,
        }
    }

    pub fn from_with_paths(
        host: String,
        port: u64,
        storage_root: PathBuf,
        history_path: PathBuf,
    ) -> Self {
        Self {
            numbers_of_connexion: Mutex::new(0u128),
            host,
            port,
            storage_root,
            history_path,
        }
    }

    pub fn get_adress(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub async fn get_number_of_connexion(&self) -> u128 {
        let res = self.numbers_of_connexion.lock().await;
        *res
    }

    async fn manage_customer(
        stream: TcpStream,
        storage_root: PathBuf,
        history_path: PathBuf,
    ) -> Result<(), FileVaultError> {
        let mut reader = BufReader::new(stream);
        let packet = Packet::from_bytes(&mut reader).await?;

        let mut file = open_file_write(
            &storage_root
                .join(packet.get_name().as_str())
                .to_string_lossy()
                .into_owned(),
            false,
        ).await
        .map_err(|e| FileVaultError::FileOpeningError(e.to_string()))?;

        Self::write_data(&mut reader, &mut file, packet.data_size()).await?;
        let _ = Self::update_history(packet.get_name(), packet.data_size(), history_path).await;

        let pool = connexion_db()
            .await
            .map_err(|e| FileVaultError::DatabaseError(e.to_string()))?;

        insert_file(pool, &packet.get_name(), packet.data_size())
            .await
            .map_err(|e| FileVaultError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn add_new_connexion(&mut self) {
        let mut number_of_connexion = self.numbers_of_connexion.lock().await;
        *number_of_connexion = number_of_connexion.saturating_add(1);
    }

    async fn update_history(
        filename: String,
        size: u64,
        history_path: PathBuf,
    ) -> Result<(), FileVaultError> {
        let now = Utc::now().format("%Y-%m-%d %H:%M:%S");
        let entry = format!("[SERVER]: {now} UCT  -  {filename} {size} - bytes");

        let file_exist = history_path.exists();
        let mut file = open_file_append(&history_path.to_string_lossy().to_owned(), file_exist)
            .await
            .map_err(|e| FileVaultError::FileOpeningError(e.to_string()))?;

        add_line_in_file(&mut file, entry.bytes().collect())
            .await
            .map_err(|e| FileVaultError::FileWritingError(e.to_string()))?;

        Ok(())
    }

    async fn write_data(
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
                .await
                .map_err(|_| FileVaultError::MissingPayload)?;

            if read_bytes == 0 {
                return Err(FileVaultError::MissingPayload);
            }

            file.write_all(&chunk[..read_bytes]).await
                .map_err(|e| FileVaultError::FileWritingError(e.to_string()))?;
            hasher.update(&chunk[..read_bytes]);
            remaining -= read_bytes as u64;
        }

        let digest = hasher.finalize();
        let mut computed_hash = [0u8; 32];
        computed_hash.copy_from_slice(&digest);

        let mut received_hash = [0u8; 32];
        reader
            .read_exact(&mut received_hash).await
            .map_err(|_| FileVaultError::MissingPayload)?;

        if received_hash != computed_hash {
            return Err(FileVaultError::HashMismatch);
        }

        file.flush().await
            .map_err(|e| FileVaultError::FileWritingError(e.to_string()))?;

        Ok(())
    }

    pub fn listening(&mut self, timeout: Duration) -> Result<(), FileVaultError> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| FileVaultError::ConnectionError(e.to_string()))?;

        runtime.block_on(self.listening_async(timeout))
    }

    pub async fn listening_async(&mut self, timeout: Duration) -> Result<(), FileVaultError> {
        let listener = TcpListener::bind(self.get_adress())
            .await
            .map_err(|e| FileVaultError::ConnectionError(e.to_string()))?;

        let mut inactivity_deadline = Instant::now() + timeout;

        println!("[SERVER]: Listening on port {}", self.port);

        loop {
            tokio::select! {
                accept_result = listener.accept() => {
                    match accept_result {
                        Ok((stream, _)) => {
                            inactivity_deadline = Instant::now() + timeout;
                            self.add_new_connexion().await;
                            let storage_root = self.storage_root.clone();
                            let history_path = self.history_path.clone();

                            tokio::spawn(async move {
                                let _ = Self::manage_customer(stream, storage_root, history_path).await;
                            });
                        }
                        Err(e) => {
                            return Err(FileVaultError::ConnectionError(e.to_string()));
                        }
                    }
                }
                _ = sleep_until(inactivity_deadline) => break,
            }
        }

        

        Ok(())
    }
}