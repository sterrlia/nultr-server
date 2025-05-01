use std::collections::HashMap;
use std::ops::ControlFlow;
use std::sync::Arc;

use axum::extract::ws;

use futures::lock::Mutex;
//allows to split the websocket stream into separate TX and RX branches
use futures::stream::StreamExt;

use futures::SinkExt;
use futures::stream::{SplitSink, SplitStream};
use tokio::sync::mpsc::{self, UnboundedReceiver};

use crate::websocket_handler::message::{InputWebsocketMessage, OutputWebsocketMessage};

pub struct MessageFromUser {
    pub sender_username: String,
    pub content: String,
}

pub type Tx = mpsc::UnboundedSender<MessageFromUser>;
pub type UserMessageSenderMap = Arc<Mutex<HashMap<String, Tx>>>; // username -> sender

#[derive()]
pub struct Handler {
    pub username: String,
    pub user_message_sender_map: UserMessageSenderMap,
    pub user_message_receiver: UnboundedReceiver<MessageFromUser>,
    pub ws_sender: SplitSink<ws::WebSocket, ws::Message>,
    pub ws_receiver: SplitStream<ws::WebSocket>,
}

enum ReceivedMessage {
    UserMessage(MessageFromUser),
    WebsocketMessage(ws::Message),
}

impl Handler {
    async fn get_message(&mut self) -> Option<ReceivedMessage> {
        tokio::select! {
            input = self.user_message_receiver.recv() => {
                if let Some(message) = input {
                    Some(ReceivedMessage::UserMessage(message))
                } else {
                    None
                }
            },
            input = self.ws_receiver.next() => {
                if let Some(Ok(message)) = input {
                    Some(ReceivedMessage::WebsocketMessage(message))
                } else {
                    None
                }
            }
        }
    }

    pub async fn listen(&mut self) {
        while let Some(message) = self.get_message().await {
            let process_result = match message {
                ReceivedMessage::UserMessage(user_message) => {
                    self.process_user_message(user_message).await
                }
                ReceivedMessage::WebsocketMessage(ws_message) => {
                    self.process_ws_message(ws_message).await
                }
            };

            if process_result == ControlFlow::Break(()) {
                break;
            }
        }
    }

    async fn process_user_message(&mut self, message: MessageFromUser) -> ControlFlow<(), ()> {
        let message_to_send = OutputWebsocketMessage::MessageFromUser {
            username: message.sender_username,
            content: message.content,
        };

        if self.ws_sender.send(message_to_send.into()).await.is_err() {
            return ControlFlow::Break(());
        }

        ControlFlow::Continue(())
    }

    async fn process_ws_message(&mut self, message: ws::Message) -> ControlFlow<(), ()> {
        match message {
            ws::Message::Text(t) => {
                let message_content = t.to_string();
                self.process_text_message(message_content).await;
            }
            _ => {
                let message_to_send = OutputWebsocketMessage::WrongFormat;

                if self.ws_sender.send(message_to_send.into()).await.is_err() {
                    return ControlFlow::Break(());
                }
            }
        }
        ControlFlow::Continue(())
    }

    async fn process_text_message(&mut self, message: String) -> ControlFlow<(), ()> {
        let result_of_parsing: Result<InputWebsocketMessage, serde_json::Error> =
            serde_json::from_str(message.as_str());

        let message_to_send = match result_of_parsing {
            Ok(input_message) => match input_message {
                InputWebsocketMessage::Health => OutputWebsocketMessage::Ok,
                InputWebsocketMessage::MessageToUser { username, content } => {
                    let message_to_send = MessageFromUser {
                        sender_username: self.username.clone(),
                        content 
                    };

                    self.send_message_to_user(username, message_to_send)
                        .await
                }
            },
            Err(_) => OutputWebsocketMessage::WrongJsonFormat,
        };

        if self.ws_sender.send(message_to_send.into()).await.is_err() {
            return ControlFlow::Break(());
        }

        return ControlFlow::Continue(());
    }

    async fn send_message_to_user(&mut self, username: String, message: MessageFromUser) -> OutputWebsocketMessage {
        let found_user_sender = self
            .user_message_sender_map
            .lock()
            .await
            .get(username.as_str())
            .cloned();

        if let Some(user_sender) = found_user_sender {
            return match user_sender.send(message) {
                Ok(_) => OutputWebsocketMessage::Ok,
                Err(error) => {
                    tracing::error!("Channel error: {}", error.to_string());
                    OutputWebsocketMessage::ChannelError
                }
            };
        }

        OutputWebsocketMessage::UserNotFound
    }
}
