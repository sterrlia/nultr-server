use anyhow::anyhow;
use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::ws;

use chrono::{NaiveDateTime, Utc};
use futures::stream::StreamExt;

use futures::SinkExt;
use futures::stream::{SplitSink, SplitStream};
use sea_orm::ActiveValue::Set;
use tokio::sync::Mutex;
use tokio::sync::mpsc::{self, UnboundedReceiver};

use crate::db::entity::messages;
use crate::state::MessageFromUser;
use crate::{auth, state};

use super::message::{Request, Response};

pub struct Controller {
    pub mutex_state: Arc<Mutex<state::MutexState>>,
    pub service_state: state::ServiceState,
    pub claims: auth::jwt::Claims,
    pub user_message_receiver: UnboundedReceiver<MessageFromUser>,
    pub ws_sender: SplitSink<ws::WebSocket, ws::Message>,
    pub ws_receiver: SplitStream<ws::WebSocket>,
}

pub enum ReceivedMessage {
    FromOtherThread(MessageFromUser),
    FromWebsocket(ws::Message),
}

impl Controller {
    pub async fn get_message(&mut self) -> Option<ReceivedMessage> {
        tokio::select! {
            input = self.user_message_receiver.recv() => {
                if let Some(message) = input {
                    Some(ReceivedMessage::FromOtherThread(message))
                } else {
                    None
                }
            },
            input = self.ws_receiver.next() => {
                if let Some(Ok(message)) = input {
                    Some(ReceivedMessage::FromWebsocket(message))
                } else {
                    None
                }
            }
        }
    }

    pub async fn process(&mut self, message: ReceivedMessage) -> anyhow::Result<()> {
        match message {
            ReceivedMessage::FromOtherThread(user_message) => {
                self.process_user_message(user_message).await
            }
            ReceivedMessage::FromWebsocket(ws_message) => self.process_ws_message(ws_message).await,
        }
    }

    async fn process_user_message(&mut self, message: MessageFromUser) -> anyhow::Result<()> {
        let message_to_send = Response::Message {
            user_id: message.from_user_id.clone(),
            content: message.content.clone(),
        };

        let message_model = messages::ActiveModel {
            from_user_id: Set(message.from_user_id),
            to_user_id: Set(self.claims.user_id),
            content: Set(message.content),
            created_at: Set(Utc::now().naive_utc()),
            ..Default::default()
        };

        self.service_state
            .message_repository
            .insert(message_model)
            .await?;

        self.ws_sender
            .send(message_to_send.into())
            .await
            .map_err(|err| anyhow!(err))
    }

    async fn process_ws_message(&mut self, message: ws::Message) -> anyhow::Result<()> {
        match message {
            ws::Message::Text(t) => {
                let message_content = t.to_string();
                self.process_text_message(message_content).await
            }
            _ => {
                let message_to_send = Response::WrongFormat;

                self.ws_sender
                    .send(message_to_send.into())
                    .await
                    .map_err(|err| anyhow!(err))
            }
        }
    }

    async fn process_text_message(&mut self, message: String) -> anyhow::Result<()> {
        let result_of_parsing: Result<Request, serde_json::Error> =
            serde_json::from_str(message.as_str());

        let message_to_send = match result_of_parsing {
            Ok(input_message) => match input_message {
                Request::MessageToUser { user_id, content } => {
                    let message_to_send = MessageFromUser {
                        from_user_id: self.claims.user_id,
                        content,
                    };

                    self.send_message_to_user(user_id, message_to_send).await?
                }
            },
            Err(error) => {
                tracing::warn!("Request parsing error: {error}");

                Response::WrongJsonFormat
            }
        };

        self.ws_sender
            .send(message_to_send.into())
            .await
            .map_err(|err| anyhow!(err))
    }

    async fn send_message_to_user(
        &mut self,
        user_id: i32,
        message: MessageFromUser,
    ) -> anyhow::Result<Response> {
        let found_user_sender = self
            .mutex_state
            .lock()
            .await
            .user_message_sender_map
            .get(&user_id)
            .cloned();

        if let Some(user_sender) = found_user_sender {
            user_sender.send(message).map_err(|err| anyhow!(err))?;

            Ok(Response::Ok)
        } else {
            Ok(Response::UserNotFound)
        }
    }
}
