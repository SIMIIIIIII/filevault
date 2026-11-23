use std::path::PathBuf;

use crate::{
    cli_helpers::{get_history_test, runtime_directories, RuntimeMode},
    kafka::{handlers::handlers, producer::FileEvent},
};
use rdkafka::{
    config::ClientConfig,
    consumer::{CommitMode, Consumer, StreamConsumer},
    Message,
};

pub struct KafkaConsumer {
    consumer: StreamConsumer,
    topic: String,
    pub history_path: PathBuf,
}

impl KafkaConsumer {
    pub fn new(
        brokers: &str,
        group_id: &str,
        topic: &str,
        runtime_mode: RuntimeMode,
    ) -> anyhow::Result<Self> {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("group.id", group_id)
            .set("auto.offset.reset", "earliest")
            .set("enable.auto.commit", "false")
            .create()?;

        consumer.subscribe(&[topic])?;

        Ok(Self {
            consumer,
            topic: topic.to_string(),
            history_path: if runtime_mode == RuntimeMode::Production {
                PathBuf::from("log_files/history.log")
            } else {
                let (log_root, _) = runtime_directories(RuntimeMode::Test);
                PathBuf::from(get_history_test(log_root))
            },
        })
    }

    pub async fn start(&self) -> anyhow::Result<()> {
        loop {
            let strm = self.consumer.recv().await;

            if strm.is_err() {
                continue;
            }

            let message = strm.unwrap();

            let Some(payload) = message.payload() else {
                tracing::warn!(topic = %self.topic, "Kafka message without payload");
                continue;
            };

            let get_event: Result<FileEvent, serde_json::Error> = serde_json::from_slice(payload);
            if get_event.is_err() {
                tracing::warn!(topic = %self.topic, "Invalid JSON");
                continue;
            }

            let event = get_event.unwrap();
            tracing::info!(topic = %self.topic, event = ?event, "Kafka event received");

            for _ in 0..5 {
                if handlers(self.history_path.clone(), event.clone())
                    .await
                    .is_err()
                {
                    continue;
                }
                break;
            }

            self.consumer.commit_message(&message, CommitMode::Sync)?;
        }
    }

    pub fn stop(&self) {
        tracing::info!(topic = %self.topic, "[KAFKA]: Fermeture du consumer");
        self.consumer.unsubscribe();
    }
}
