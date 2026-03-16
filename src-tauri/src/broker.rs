use rskafka::client::{partition::UnknownTopicHandling, ClientBuilder};
use rskafka::record::Record;
use std::sync::Arc;
use time::OffsetDateTime;

pub async fn send_intel(payload: String) -> Result<(), String> {
    let connection = "localhost:19092".to_string();
    let client = ClientBuilder::new(vec![connection])
        .build()
        .await
        .map_err(|e| format!("Broker connect failed: {}", e))?;

    let controller = client.controller_client().map_err(|e| e.to_string())?;
    // Пытаемся создать топик (игнорируем ошибку, если он уже существует)
    let _ = controller.create_topic("osint-raw-intel", 1, 1, 5000).await;

    // Исправлено: добавлен UnknownTopicHandling::Retry и вызов .await
    let partition_client = Arc::new(
        client
            .partition_client("osint-raw-intel", 0, UnknownTopicHandling::Retry)
            .await
            .map_err(|e| format!("Partition error: {}", e))?,
    );

    let record = Record {
        key: None::<Vec<u8>>, // Исправлено: Явное указание типа для пустого ключа
        value: Some(payload.into_bytes()),
        headers: Default::default(),
        timestamp: OffsetDateTime::now_utc(), // Исправлено: Используем time вместо chrono
    };

    partition_client
        .produce(
            vec![record],
            rskafka::client::partition::Compression::NoCompression,
        )
        .await
        .map_err(|e| format!("Produce error: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn test_broker_connection(message: String) -> Result<String, String> {
    match send_intel(message.clone()).await {
        Ok(_) => Ok(format!(
            "Message successfully sent to Redpanda: {}",
            message
        )),
        Err(e) => Err(e),
    }
}
