use anyhow::anyhow;
use nultr_shared_lib::request::{
    WsErrorResponse, WsMessageRequest, WsMessageResponse, WsOkResponse, WsRequest, WsResponse,
};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

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
        let message_response = WsMessageResponse {
            id: Uuid::new_v4(),
            user_id: message.from_user_id.clone(),
            content: message.content.clone(),
            created_at: Utc::now().naive_utc(),
        };

        let message_to_send = WsResponse::Ok(WsOkResponse::Message(message_response));

        let serialize_result: Result<String, serde_json::Error> =
            serde_json::to_string(&message_to_send);

        let ws_response = match serialize_result {
            Ok(serialized_data) => serialized_data,
            Err(error) => {
                tracing::error!("Response serialization error {:?}", error);

                let message_to_send = WsResponse::Err(WsErrorResponse::Fatal);

                serde_json::to_string(&message_to_send).map_err(|err| anyhow!(err))?
            }
        };

        self.ws_sender
            .send(ws::Message::Text(ws_response.into()))
            .await
            .map_err(|err| anyhow!(err))
    }

    async fn process_ws_message(&mut self, message: ws::Message) -> anyhow::Result<()> {
        let message_to_send = if let ws::Message::Text(text) = message {
            let message_content = text.to_string();
            self.process_text_message(message_content).await.into()
        } else {
            WsResponse::Err(WsErrorResponse::WrongFormat)
        };

        let serialize_result: Result<String, serde_json::Error> =
            serde_json::to_string(&message_to_send);

        let ws_response = match serialize_result {
            Ok(serialized_data) => serialized_data,
            Err(error) => {
                tracing::error!("Response serialization error {:?}", error);

                let message_to_send = WsResponse::Err(WsErrorResponse::Fatal);

                serde_json::to_string(&message_to_send).map_err(|err| anyhow!(err))?
            }
        };

        self.ws_sender
            .send(ws::Message::Text(ws_response.into()))
            .await
            .map_err(|err| anyhow!(err))
    }

    async fn process_text_message(
        &mut self,
        message: String,
    ) -> Result<WsOkResponse, WsErrorResponse> {
        let request: WsRequest = serde_json::from_str(message.as_str())
            .inspect_err(|err| tracing::warn!("Request parsing error: {:?}", err))
            .map_err(|_| WsErrorResponse::WrongJsonFormat)?;

        match request {
            WsRequest::Message(message) => self.send_message_to_user(message).await,
        }
    }

    async fn send_message_to_user(
        &mut self,
        message: WsMessageRequest,
    ) -> Result<WsOkResponse, WsErrorResponse> {
        let user = self
            .service_state
            .user_repository
            .get_by_id(message.user_id)
            .await?;

        if user == None {
            return Err(WsErrorResponse::UserNotFound);
        }

        let message_model = messages::Model {
            id: message.id,
            from_user_id: self.claims.user_id,
            to_user_id: message.user_id,
            content: message.content.clone(),
            created_at: Utc::now().naive_utc(),
        };

        self.service_state
            .message_repository
            .insert(message_model)
            .await?;

        let found_user_sender = self
            .mutex_state
            .lock()
            .await
            .user_message_sender_map
            .get(&message.user_id)
            .cloned();

        if let Some(user_sender) = found_user_sender {
            let message_to_send = MessageFromUser {
                id: message.id,
                from_user_id: self.claims.user_id,
                content: message.content,
            };

            user_sender
                .send(message_to_send)
                .map_err(|err| anyhow!(err))?;
        }

        Ok(WsOkResponse::MessageSent)
    }
}
