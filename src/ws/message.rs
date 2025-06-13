use axum::extract::ws;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct MessageResponse {
    pub id: Uuid,
    pub user_id: i32,
    pub content: String,
    pub created_at: NaiveDateTime
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum Response {
    MessageSent,
    Message(MessageResponse),
    WrongFormat,
    WrongJsonFormat,
    UserNotFound,
    ChannelError,
}

impl From<Response> for ws::Message {
    fn from(msg: Response) -> Self {
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
pub struct MessageRequest {
    pub id: Uuid,
    pub user_id: i32,
    pub content: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum Request {
    Message(MessageRequest),
}
