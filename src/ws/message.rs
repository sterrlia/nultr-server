use axum::extract::ws;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum Response {
    Ok,
    Message {
        user_id: i32,
        content: String
    },
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
#[serde(tag = "type")]
pub enum Request {
    MessageToUser { user_id: i32, content: String },
}

