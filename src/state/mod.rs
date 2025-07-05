use std::{collections::HashMap, sync::Arc};

use nultr_shared_lib::request::WsMarkMessagesReadRequest;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::{auth, db::{self, repository::{MessageRepository, RoomRepository, UserRepository}}};

pub type MessagesReadEvent = WsMarkMessagesReadRequest;

#[derive(Clone)]
pub enum ThreadEvent {
    UserMessage(UserMessage),
    MessagesRead(MessagesReadEvent),
}

#[derive(Clone)]
pub struct UserMessage {
    pub uuid: Uuid,
    pub from_user_id: i32,
    pub content: String,
}

pub struct MutexState {
    pub user_message_sender_map: HashMap<i32, mpsc::UnboundedSender<ThreadEvent>>,
}

#[derive(Clone)]
pub struct ServiceState {
    pub user_repository: UserRepository,
    pub room_repository: RoomRepository,
    pub message_repository: MessageRepository,
    pub password_hasher: auth::PasswordHasher,
    pub jwt_encoder: auth::jwt::Encoder,
}

impl Default for ServiceState {
    fn default() -> Self {
        let lazy_connector = Arc::new(db::LazyConnector::default());
        let room_repository = RoomRepository {
            lazy_connector: lazy_connector.clone(),
        };

        let user_repository = UserRepository {
            lazy_connector: lazy_connector.clone(),
        };

        let message_repository = MessageRepository { lazy_connector };

        let password_hasher = auth::PasswordHasher::default();

        let jwt_encoder = auth::jwt::Encoder::default();

        Self {
            user_repository,
            room_repository,
            message_repository,
            password_hasher,
            jwt_encoder,
        }
    }
}

pub struct CliState {
    pub password_hasher: auth::PasswordHasher,
    pub user_repository: UserRepository,
}

impl Default for CliState {
    fn default() -> Self {
        let lazy_connector = Arc::new(db::LazyConnector::default());

        let user_repository = UserRepository {
            lazy_connector: lazy_connector.clone(),
        };

        let password_hasher = auth::PasswordHasher::default();

        Self {
            password_hasher,
            user_repository,
        }
    }
}
