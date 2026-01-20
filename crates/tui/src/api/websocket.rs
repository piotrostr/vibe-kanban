use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::state::Task;

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum WsMessage {
    JsonPatch {
        #[serde(rename = "JsonPatch")]
        patches: Vec<json_patch::PatchOperation>,
    },
    Finished {
        finished: bool,
    },
}

pub type TaskUpdateSender = mpsc::Sender<Vec<Task>>;
pub type TaskUpdateReceiver = mpsc::Receiver<Vec<Task>>;

pub struct TaskStreamConnection {
    sender: TaskUpdateSender,
}

impl TaskStreamConnection {
    pub fn new(sender: TaskUpdateSender) -> Self {
        Self { sender }
    }

    pub async fn connect(
        base_url: &str,
        project_id: &str,
        sender: TaskUpdateSender,
    ) -> Result<()> {
        let ws_url = format!(
            "{}/tasks/stream/ws?project_id={}",
            base_url.replace("http://", "ws://"),
            project_id
        );

        tracing::info!("Connecting to WebSocket: {}", ws_url);

        let (ws_stream, _) = connect_async(&ws_url).await?;
        let (mut write, mut read) = ws_stream.split();

        // Initialize with empty tasks - the server will send the initial state via patches
        let mut tasks: Vec<Task> = Vec::new();

        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    match serde_json::from_str::<WsMessage>(&text) {
                        Ok(WsMessage::JsonPatch { patches }) => {
                            // Apply patches to the tasks array
                            let mut json_value = serde_json::to_value(&tasks)?;

                            for patch in patches {
                                if let Err(e) = json_patch::patch(&mut json_value, &[patch]) {
                                    tracing::warn!("Failed to apply patch: {}", e);
                                }
                            }

                            // Deserialize back to tasks
                            match serde_json::from_value::<Vec<Task>>(json_value) {
                                Ok(updated_tasks) => {
                                    tasks = updated_tasks;
                                    // Send update to the app
                                    if sender.send(tasks.clone()).await.is_err() {
                                        tracing::info!("Receiver dropped, closing WebSocket");
                                        break;
                                    }
                                }
                                Err(e) => {
                                    tracing::warn!("Failed to deserialize tasks: {}", e);
                                }
                            }
                        }
                        Ok(WsMessage::Finished { finished: true }) => {
                            tracing::info!("WebSocket stream finished");
                            break;
                        }
                        Ok(WsMessage::Finished { finished: false }) => {
                            // Continue
                        }
                        Err(e) => {
                            tracing::warn!("Failed to parse WebSocket message: {} - {}", e, text);
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    tracing::info!("WebSocket closed by server");
                    break;
                }
                Ok(Message::Ping(data)) => {
                    // Respond to ping with pong
                    if let Err(e) = write.send(Message::Pong(data)).await {
                        tracing::warn!("Failed to send pong: {}", e);
                        break;
                    }
                }
                Ok(_) => {
                    // Ignore other message types
                }
                Err(e) => {
                    tracing::error!("WebSocket error: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }
}

pub fn create_task_channel() -> (TaskUpdateSender, TaskUpdateReceiver) {
    mpsc::channel(100)
}
