use rdkafka::{
    config::ClientConfig,
    producer::{FutureProducer, FutureRecord},
    util::Timeout,
};

use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::{db::File, kafka::handlers::Transaction};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum FileEvent {
    Uploaded {
        file_info: File,
        uploaded_at: String,
        user_id: i64,
    },

    Downloaded {
        user_id: i64,
        file_id: i64,
        file_info: Option<File>,
        downloaded_at: String,
    },

    Deleted {
        user_id: i64,
        file_id: i64,
        deleted_at: String,
    },

    Logged {
        user_id: i64,
        logged_in_at: String,
    },

    QuotaAtteint {
        user_id: i64,
        quota_mb: i64,
        datum: String,
    },
}

#[derive(Clone)]
pub struct KafkaProducer {
    producer: FutureProducer,
    topic: String,
}

impl KafkaProducer {
    pub fn new(brokers: &str, topic: &str) -> anyhow::Result<Self> {
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("acks", "all")
            .set("retries", "2147483647")
            .set("retry.backoff.ms", "100")
            .set("delivery.timeout.ms", "120000")
            .set("enable.idempotence", "true")
            .set("max.in.flight.requests.per.connection", "5")
            .set("compression.type", "snappy") // Compression pour les gros payloads
            .set("linger.ms", "5") // Batching pour le throughput
            .set("batch.size", "65536") // 64KB
            .create()?;
        Ok(Self {
            producer,
            topic: topic.to_string(),
        })
    }

    pub async fn publier(
        &self,
        event: &FileEvent,
        transaction: &Transaction,
    ) -> anyhow::Result<()> {
        // Clé = user_id pour garantir l"ordre par utilisateur
        let cle = match event {
            FileEvent::Uploaded { user_id, .. } => user_id.to_string(),
            FileEvent::Downloaded { user_id, .. } => user_id.to_string(),
            FileEvent::Deleted { user_id, .. } => user_id.to_string(),
            FileEvent::QuotaAtteint { user_id, .. } => user_id.to_string(),
            FileEvent::Logged { user_id, .. } => user_id.to_string(),
        };

        let payload = serde_json::to_string(event)?;

        let record = FutureRecord::to(&self.topic).key(&cle).payload(&payload);

        // Attendre l"ACK du broker (acks=all)
        if self
            .producer
            .send(record, Timeout::After(Duration::from_secs(10)))
            .await
            .is_err()
        {
            let mut outbox_lock = transaction.outbox.lock().unwrap();
            outbox_lock.push((*event).clone());
        }

        tracing::info!(event = ?event, cle = %cle, "Événement publié dans Kafka");
        Ok(())
    }
}
