use axum::{
    Json,
    extract::{self, Query},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth,
    db::{self, entity::users},
    state,
};

use super::ErrorResponse;

#[derive(Serialize)]
struct UserResponse {
    id: i32,
    username: String,
}

pub async fn get_users(
    extract::State(state): extract::State<state::ServiceState>,
    _claims: auth::jwt::Claims,
) -> Result<impl IntoResponse, ErrorResponse> {
    let users = state.user_repository.get_all().await?;

    let user_response: Vec<UserResponse> = users
        .iter()
        .map(|user| UserResponse {
            id: user.id,
            username: user.username.clone(),
        })
        .collect();

    Ok((StatusCode::OK, Json(user_response)).into_response())
}

#[derive(Deserialize)]
pub struct GetMessagesRequest {
    pub user_id: i32,
    pub pagination: db::Pagination,
}

#[derive(Serialize)]
struct MessageResponse {
    pub id: Uuid,
    pub user_id: i32,
    pub content: String,
    pub created_at: NaiveDateTime,
}

pub async fn get_messages(
    Query(request): Query<GetMessagesRequest>,
    extract::State(state): extract::State<state::ServiceState>,
    claims: auth::jwt::Claims,
) -> Result<impl IntoResponse, ErrorResponse> {
    state
        .user_repository
        .get_by_id(request.user_id)
        .await?
        .ok_or(ErrorResponse::UserNotFound)?;

    let messages = state
        .message_repository
        .get_messages_between_users(request.user_id, claims.user_id, request.pagination)
        .await?;

    let message_response: Vec<MessageResponse> = messages
        .iter()
        .map(|message| MessageResponse {
            id: message.id,
            user_id: message.from_user_id,
            content: message.content.clone(),
            created_at: message.created_at,
        })
        .collect();

    Ok((StatusCode::OK, Json(message_response)))
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub user_id: i32,
    pub token: String,
}

pub async fn login(
    extract::State(state): extract::State<state::ServiceState>,
    Json(input): Json<LoginRequest>,
) -> Result<impl IntoResponse, ErrorResponse> {
    let user_result = state
        .user_repository
        .get_by_username(input.username)
        .await?;

    if let Some(user) = user_result {
        let verified = state
            .password_hasher
            .verify_password(input.password.as_str(), user.password_hash.as_str());

        if verified {
            let token = state.jwt_encoder.encode(user.id)?;
            let response = LoginResponse {
                user_id: user.id,
                token
            };

            Ok((StatusCode::OK, Json(response)))
        } else {
            Err(ErrorResponse::AccessDenied)
        }
    } else {
        Err(ErrorResponse::AccessDenied)
    }
}
