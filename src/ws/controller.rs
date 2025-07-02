use anyhow::anyhow;
use nultr_shared_lib::request::{
    WsErrorResponse, WsMessageRequest, WsMessageResponse, WsOkResponse, WsRequest, WsResponse,
};
use std::sync::Arc;
use uuid::Uuid;

use axum::extract::ws;

use chrono::Utc;
use futures::stream::StreamExt;

use futures::SinkExt;
use futures::stream::{SplitSink, SplitStream};
use sea_orm::ActiveValue::Set;
use tokio::sync::Mutex;
use tokio::sync::mpsc::{self, UnboundedReceiver};

use crate::db::{RepositoryTrait, entity::messages};
use crate::state::{ThreadEvent, UserMessage};
use crate::{auth, state};

pub struct Controller {
    pub mutex_state: Arc<Mutex<state::MutexState>>,
    pub service_state: state::ServiceState,
    pub claims: auth::jwt::Claims,
    pub user_message_receiver: UnboundedReceiver<ThreadEvent>,
    pub ws_sender: SplitSink<ws::WebSocket, ws::Message>,
    pub ws_receiver: SplitStream<ws::WebSocket>,
}

pub enum ReceivedEvent {
    FromOtherThread(ThreadEvent),
    FromWebsocket(ws::Message),
}

impl Controller {
    pub async fn get_message(&mut self) -> Option<ReceivedEvent> {
        tokio::select! {
            input = self.user_message_receiver.recv() => {
                if let Some(message) = input {
                    Some(ReceivedEvent::FromOtherThread(message))
                } else {
                    None
                }
            },
            input = self.ws_receiver.next() => {
                if let Some(Ok(message)) = input {
                    Some(ReceivedEvent::FromWebsocket(message))
                } else {
                    None
                }
            }
        }
    }

    pub async fn process(&mut self, message: ReceivedEvent) -> anyhow::Result<()> {
        match message {
            ReceivedEvent::FromOtherThread(event) => self.process_thread_event(event).await,
            ReceivedEvent::FromWebsocket(ws_message) => {
                if let ws::Message::Text(text) = ws_message {
                    let request: Result<WsRequest, serde_json::Error> =
                        serde_json::from_str(text.as_str());

                    match request {
                        Ok(request) => self.process_ws_request(request).await,
                        Err(error) => {
                            tracing::warn!("Request parsing error: {:?}", error);

                            self.send_ws_response(WsResponse::Err(WsErrorResponse::WrongFormat))
                                .await
                        }
                    }
                } else {
                    tracing::warn!("Wrong request format");

                    self.send_ws_response(WsResponse::Err(WsErrorResponse::WrongFormat))
                        .await
                }
            }
        }
    }

    async fn process_thread_event(&mut self, event: ThreadEvent) -> anyhow::Result<()> {
        match event {
            ThreadEvent::UserMessage(message) => {
                let response = WsOkResponse::Message(WsMessageResponse {
                    id: Uuid::new_v4(),
                    user_id: message.from_user_id.clone(),
                    content: message.content.clone(),
                    created_at: Utc::now().naive_utc(),
                });

                self.send_ws_response(WsResponse::Ok(response)).await
            }
            ThreadEvent::MessageRead(uuid) => {
                let response = WsOkResponse::MessageRead(uuid);
                self.send_ws_response(WsResponse::Ok(response)).await
            }
        }
    }

    async fn process_ws_request(&mut self, request: WsRequest) -> anyhow::Result<()> {
        match request {
            WsRequest::Message(message) => self.send_message_to_user(message).await,
            WsRequest::MessageRead(message_uuid) => self.mark_message_read(message_uuid).await,
        }
    }

    async fn mark_message_read(&mut self, message_uuid: Uuid) -> anyhow::Result<()> {
        let found_message = self
            .service_state
            .message_repository
            .get_message_by_uuid(message_uuid)
            .await?;

        match found_message {
            Some(message) => {
                if !message.read {
                    let mut active_model: messages::ActiveModel = message.into();
                    active_model.read = Set(true);
                    self.service_state.message_repository.update(active_model).await?;
                }
            }
            None => {
                self.send_ws_response(WsResponse::Err(WsErrorResponse::MessageNotFound(
                    message_uuid,
                )))
                .await?
            }
        }

        Ok(())
    }

    async fn send_message_to_user(&mut self, request: WsMessageRequest) -> anyhow::Result<()> {
        let room = self
            .service_state
            .room_repository
            .get_by_id(request.room_id)
            .await?;

        if room == None {
            tracing::error!("Room not found by id: {}", request.room_id);

            return self
                .send_ws_response(WsResponse::Err(WsErrorResponse::UserNotFound))
                .await;
        }

        let room_users = self
            .service_state
            .room_repository
            .get_users_by_room(request.room_id)
            .await?;

        let user_is_member = room_users.iter().any(|user| user.id == self.claims.user_id);

        if !user_is_member {
            tracing::error!(
                "User is not member of room: {}, {}",
                request.room_id,
                self.claims.user_id
            );

            return self
                .send_ws_response(WsResponse::Err(WsErrorResponse::NotMemberOfRoom))
                .await;
        }

        let save_to_db = async {
            let message_model = messages::ActiveModel {
                uuid: Set(request.uuid.clone()),
                user_id: Set(self.claims.user_id),
                room_id: Set(request.room_id),
                content: Set(request.content.clone()),
                created_at: Set(Utc::now().naive_utc()),
                ..Default::default()
            };

            self.service_state
                .message_repository
                .insert(message_model)
                .await
        };

        let send_events = async {
            let thread_event = state::ThreadEvent::UserMessage(UserMessage {
                uuid: request.uuid,
                from_user_id: self.claims.user_id,
                content: request.content.clone(),
            });

            for user in room_users {
                self.send_thread_event(user.id, thread_event.clone())
                    .await?;
            }

            Ok::<(), anyhow::Error>(())
        };

        // TODO: run simultaneously ?
        save_to_db.await?;
        send_events.await?;

        self.send_ws_response(WsResponse::Ok(WsOkResponse::MessageReceived(request.uuid)))
            .await
    }

    async fn send_thread_event(&self, user_id: i32, event: ThreadEvent) -> anyhow::Result<()> {
        let found_user_sender = self
            .mutex_state
            .lock()
            .await
            .user_message_sender_map
            .get(&user_id)
            .cloned();

        if let Some(user_sender) = found_user_sender {
            user_sender.send(event).map_err(|err| anyhow!(err))?;
        }

        Ok(())
    }

    async fn send_ws_response(&mut self, response: WsResponse) -> anyhow::Result<()> {
        let serialize_result: Result<String, serde_json::Error> = serde_json::to_string(&response);

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
}
