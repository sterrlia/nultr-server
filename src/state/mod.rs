use std::{collections::HashMap, sync::Arc};

use tokio::sync::mpsc;

use crate::{auth, db};

pub struct MessageFromUser {
    pub from_user_id: i32,
    pub content: String,
}

pub struct MutexState {
    pub user_message_sender_map: HashMap<i32, mpsc::UnboundedSender<MessageFromUser>>,
}

#[derive(Clone)]
pub struct ServiceState {
    pub user_repository: db::UserRepository,
    pub message_repository: db::MessageRepository,
    pub password_hasher: auth::PasswordHasher,
    pub jwt_encoder: auth::jwt::Encoder,
}

impl Default for ServiceState {
    fn default() -> Self {
        let lazy_connector = Arc::new(db::LazyConnector::default());

        let user_repository = db::UserRepository {
            lazy_connector: lazy_connector.clone(),
        };

        let message_repository = db::MessageRepository { lazy_connector };

        let password_hasher = auth::PasswordHasher::default();

        let jwt_encoder = auth::jwt::Encoder::default();

        Self {
            user_repository,
            message_repository,
            password_hasher,
            jwt_encoder,
        }
    }
}

pub struct CliState {
    pub password_hasher: auth::PasswordHasher,
    pub user_repository: db::UserRepository,
}

impl Default for CliState {
    fn default() -> Self {
        let lazy_connector = Arc::new(db::LazyConnector::default());

        let user_repository = db::UserRepository {
            lazy_connector: lazy_connector.clone(),
        };

        let password_hasher = auth::PasswordHasher::default();

        Self { password_hasher, user_repository }
    }
}
