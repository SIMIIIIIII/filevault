use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

//use anyhow::Ok;

use crate::{
    files::{add_line_in_file, open_file_append},
    kafka::producer::{FileEvent, KafkaProducer},
};

pub async fn add_to_history(history_path: PathBuf, message: String) -> anyhow::Result<()> {
    let file_exist = history_path.exists();

    let mut file = open_file_append(&history_path.to_string_lossy(), file_exist).await?;

    add_line_in_file(&mut file, message.bytes().collect()).await?;
    println!("{message}");

    Ok(())
}

pub async fn handlers(history_path: PathBuf, event: FileEvent) -> anyhow::Result<()> {
    let message = match event.clone() {
        FileEvent::Uploaded {
            user_id,
            file_id,
            uploaded_at,
        } => {
            format!("[SERVER]: file '{file_id} uploaded by '{user_id}' on '{uploaded_at}'")
        }

        FileEvent::Deleted {
            user_id,
            file_id,
            deleted_at,
        } => {
            format!("[SERVER]: file '{file_id} deleted by '{user_id}' on '{deleted_at}'")
        }

        FileEvent::Downloaded {
            user_id,
            file_id,
            downloaded_at,
        } => {
            format!("[SERVER]: file '{file_id} downloaded by '{user_id}' on '{downloaded_at}'")
        }

        FileEvent::Logged {
            user_id,
            logged_in_at,
        } => {
            format!("[SERVER]: user '{user_id}' logged in on '{logged_in_at}'")
        }

        FileEvent::QuotaAtteint {
            user_id,
            quota_mb,
            datum,
        } => {
            format!("[SERVER]: user '{user_id}' reached the quota '{quota_mb}' on '{datum}'")
        }
    };

    add_to_history(history_path, message).await?;
    Ok(())
}

#[derive(Clone)]
pub struct Transaction {
    in_progress: Arc<Mutex<bool>>,
    pub outbox: Arc<Mutex<Vec<FileEvent>>>,
    pub kafka: KafkaProducer,
}

impl Transaction {
    pub fn new(kafka: KafkaProducer) -> anyhow::Result<Self> {
        Ok(Transaction {
            in_progress: Arc::new(Mutex::new(false)),
            outbox: Arc::new(Mutex::new(vec![])),
            kafka,
        })
    }

    fn add(&self, event: FileEvent) -> anyhow::Result<()> {
        let mut vec_lock = self.outbox.lock().unwrap();
        vec_lock.push(event);
        Ok(())
    }

    fn set_in_progress(&self) -> anyhow::Result<bool> {
        let mut progress_lock = self.in_progress.lock().unwrap();
        if *progress_lock {
            return Ok(false);
        }

        *progress_lock = true;
        Ok(true)
    }

    fn set_not_in_progress(&self) -> anyhow::Result<bool> {
        let mut progress_lock = self.in_progress.lock().unwrap();
        if *progress_lock {
            let outbox_lock = self.outbox.lock().unwrap();
            if outbox_lock.is_empty() {
                *progress_lock = false;
                return Ok(true);
            }
            return Ok(false);
        }

        Ok(true)
    }

    async fn start(&self) -> anyhow::Result<()> {
        let mut length;
        {
            let outbox_lock = self.outbox.lock().unwrap();
            length = outbox_lock.len() as u64;
        }

        let mut event: FileEvent;

        while length > 0 {
            {
                let mut outbox_lock = self.outbox.lock().unwrap();
                event = outbox_lock.remove(0);
            }

            tokio::spawn({
                let producer = self.kafka.clone();
                let transaction = self.clone();
                async move {
                    if let Err(e) = producer.publier(&event, &transaction).await {
                        tracing::error!(err = %e, "Kafka publish error");
                    }
                }
            });

            {
                let outbox_lock = self.outbox.lock().unwrap();
                length = outbox_lock.len() as u64;
            }

            if length == 0 {
                self.set_not_in_progress()?;
            }
        }
        Ok(())
    }

    pub async fn push(&self, event: FileEvent) -> anyhow::Result<()> {
        self.add(event)?;
        //let start = self.set_in_progress();

        if let Ok(can_start) = self.set_in_progress() {
            if can_start {
                self.start().await?
            }
        }

        Ok(())
    }
}
