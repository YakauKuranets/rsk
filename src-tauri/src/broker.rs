#[cfg(feature = "kafka")]
use chrono::Utc;
#[cfg(feature = "kafka")]
use rskafka::client::{partition::UnknownTopicHandling, ClientBuilder};
#[cfg(feature = "kafka")]
use rskafka::record::Record;
#[cfg(feature = "kafka")]
use std::sync::Arc;
use tokio::time::{timeout, Duration};

#[cfg(feature = "kafka")]
pub async fn send_intel(payload: String) -> Result<(), String> {
    let connection = "127.0.0.1:19092".to_string();
    let client = ClientBuilder::new(vec![connection])
        .build()
        .await
        .map_err(|e| format!("Broker connect failed: {}", e))?;

    let controller = client.controller_client().map_err(|e| e.to_string())?;
    let _ = controller.create_topic("osint-raw-intel", 1, 1, 5000).await;

    let partition_client = Arc::new(
        client
            .partition_client("osint-raw-intel", 0, UnknownTopicHandling::Retry)
            .await
            .map_err(|e| format!("Partition error: {}", e))?,
    );

    let record = Record {
        key: None::<Vec<u8>>,
        value: Some(payload.into_bytes()),
        headers: Default::default(),
        timestamp: Utc::now(),
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

#[cfg(not(feature = "kafka"))]
pub async fn send_intel(_payload: String) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub async fn test_broker_connection(message: String) -> Result<String, String> {
    #[cfg(not(feature = "kafka"))]
    {
        let _ = message;
        return Ok(
            "Kafka broker disabled at compile time (build with --features kafka to enable)."
                .to_string(),
        );
    }

    #[cfg(feature = "kafka")]
    {
        match timeout(Duration::from_secs(5), send_intel(message.clone())).await {
            Ok(Ok(_)) => Ok(format!("Message successfully sent to Redpanda: {}", message)),
            Ok(Err(e)) => Err(format!("Broker error: {}", e)),
            Err(_) => Err("Timeout: Redpanda did not respond in 5 seconds. Check if Docker container is running and accessible on 127.0.0.1:19092".to_string()),
        }
    }
}
