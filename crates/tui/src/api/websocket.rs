use std::collections::HashMap;

use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
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

/// The server sends tasks as an object: { "tasks": { "task-id": {...}, ... } }
#[derive(Debug, Default, Serialize, Deserialize)]
struct TasksState {
    #[serde(default)]
    tasks: HashMap<String, Task>,
}

pub type TaskUpdateSender = mpsc::Sender<Vec<Task>>;
pub type TaskUpdateReceiver = mpsc::Receiver<Vec<Task>>;

pub struct TaskStreamConnection;

impl TaskStreamConnection {
    pub async fn connect(
        base_url: &str,
        project_id: &str,
        sender: TaskUpdateSender,
    ) -> Result<()> {
        let ws_url = format!(
            "{}/api/tasks/stream/ws?project_id={}",
            base_url.replace("http://", "ws://"),
            project_id
        );

        tracing::info!("Connecting to WebSocket: {}", ws_url);

        let (ws_stream, _) = connect_async(&ws_url).await?;
        let (mut write, mut read) = ws_stream.split();

        // Initialize with empty state - server sends tasks as object keyed by ID
        let mut state = TasksState::default();
        let mut json_state = serde_json::to_value(&state)?;

        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    match serde_json::from_str::<WsMessage>(&text) {
                        Ok(WsMessage::JsonPatch { patches }) => {
                            // Apply patches to the state object
                            for patch in &patches {
                                if let Err(e) = json_patch::patch(&mut json_state, &[patch.clone()])
                                {
                                    tracing::warn!("Failed to apply patch: {} - {:?}", e, patch);
                                }
                            }

                            // Deserialize back to state
                            match serde_json::from_value::<TasksState>(json_state.clone()) {
                                Ok(updated_state) => {
                                    state = updated_state;
                                    // Convert map to vec and send
                                    let tasks: Vec<Task> = state.tasks.values().cloned().collect();
                                    if sender.send(tasks).await.is_err() {
                                        tracing::info!("Receiver dropped, closing WebSocket");
                                        break;
                                    }
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        "Failed to deserialize tasks state: {} - state: {}",
                                        e,
                                        json_state
                                    );
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
