use axum::{
    Json,
    body::Body,
    extract,
    http::{Response, StatusCode},
    response::IntoResponse,
};
use chrono::NaiveDateTime;
use sea_orm::ActiveValue::Set;
use serde::{Deserialize, Serialize};

use crate::{auth, db::entity::users, state};

use super::ErrorResponse;

#[derive(Serialize)]
struct UserResponse {
    id: i32,
    name: String,
}

pub async fn get_users(
    extract::State(state): extract::State<state::ServiceState>,
    _claims: auth::jwt::Claims
) -> Result<impl IntoResponse, ErrorResponse> {
    let users = state.user_repository.get_all().await?;

    let user_response: Vec<UserResponse> = users
        .iter()
        .map(|user| UserResponse {
            id: user.id,
            name: user.username.clone(),
        })
        .collect();

    Ok((StatusCode::OK, Json(user_response)).into_response())
}

#[derive(Serialize)]
struct MessageResponse {
    content: String,
    created_at: NaiveDateTime,
}

pub async fn get_messages(
    extract::Path(user_id): extract::Path<i32>,
    extract::State(state): extract::State<state::ServiceState>,
    claims: auth::jwt::Claims
) -> Result<impl IntoResponse, ErrorResponse> {
    state
        .user_repository
        .get_by_id(user_id)
        .await?
        .ok_or(ErrorResponse::UserNotFound)?;

    let messages = state
        .message_repository
        .get_messages_between_users(user_id, claims.user_id)
        .await?;

    let message_response: Vec<MessageResponse> = messages
        .iter()
        .map(|message| MessageResponse {
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
    pub access_token: String,
    pub token_type: String,
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
                access_token: token,
                token_type: "Bearer".to_string(),
            };

            Ok((StatusCode::OK, Json(response)))
        } else {
            Err(ErrorResponse::AccessDenied)
        }
    } else {
        Err(ErrorResponse::AccessDenied)
    }
}
