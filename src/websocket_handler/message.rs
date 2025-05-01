use axum::extract::ws;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum OutputWebsocketMessage {
    Ok,
    MessageFromUser {
        username: String,
        content: String
    },
    WrongFormat,
    WrongJsonFormat,
    UserNotFound,
    ChannelError,
}

impl From<OutputWebsocketMessage> for ws::Message {
    fn from(msg: OutputWebsocketMessage) -> Self {
        match serde_json::to_string(&msg) {
            Ok(json_string) => ws::Message::Text(json_string.into()),
            Err(error) => {
                tracing::error!("Serialization error: {}", error.to_string());

                ws::Message::Text("Something went wrong".into())
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum InputWebsocketMessage {
    Health,
    MessageToUser { username: String, content: String },
}
