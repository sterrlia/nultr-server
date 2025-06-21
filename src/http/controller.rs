use axum::{
    Json,
    extract::{self, Query},
};
use nultr_shared_lib::{
    request::{
        AuthenticatedUnexpectedErrorResponse, GetMessagesErrorResponse, GetMessagesRequest,
        GetMessagesResponse, GetUsersErrorResponse, GetUsersResponse, LoginErrorResponse,
        LoginRequest, LoginResponse, MessageResponse, UnexpectedErrorResponse, UserResponse,
    },
    util::MonoResult,
};
use rust_api_integrator::http::client::Response;

use crate::{
    auth,
    db::{self},
    state,
};

pub type AuthenticatedResponse<T, E> =
    MonoResult<Response<T, E, AuthenticatedUnexpectedErrorResponse>>;
pub type UnauthenticatedResponse<T, E> = MonoResult<Response<T, E, UnexpectedErrorResponse>>;

pub async fn get_users(
    extract::State(state): extract::State<state::ServiceState>,
    _claims: auth::jwt::Claims,
) -> AuthenticatedResponse<GetUsersResponse, GetUsersErrorResponse> {
    let users = state.user_repository.get_all().await?;

    let user_response = GetUsersResponse(
        users
            .iter()
            .map(|user| UserResponse {
                id: user.id,
                username: user.username.clone(),
            })
            .collect(),
    );

    Ok(Response::Ok(user_response))
}

pub async fn get_messages(
    Query(request): Query<GetMessagesRequest>,
    extract::State(state): extract::State<state::ServiceState>,
    claims: auth::jwt::Claims,
) -> AuthenticatedResponse<GetMessagesResponse, GetMessagesErrorResponse> {
    let user = state
        .user_repository
        .get_by_id(request.user_id)
        .await?;

    if user == None {
        return Err(GetMessagesErrorResponse::UserNotFound.into());
    }

    let messages = state
        .message_repository
        .get_messages_between_users(
            request.user_id,
            claims.user_id,
            db::Pagination {
                page: request.page,
                page_size: request.page_size,
            },
        )
        .await?;

    let message_response = GetMessagesResponse(
        messages
            .iter()
            .map(|message| MessageResponse {
                id: message.id,
                user_id: message.from_user_id,
                content: message.content.clone(),
                created_at: message.created_at,
            })
            .collect(),
    );

    Ok(message_response.into())
}

pub async fn login(
    extract::State(state): extract::State<state::ServiceState>,
    Json(input): Json<LoginRequest>,
) -> UnauthenticatedResponse<LoginResponse, LoginErrorResponse> {
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
                token,
            };

            Ok(response.into())
        } else {
            Err(LoginErrorResponse::AccessDenied.into())
        }
    } else {
        Err(LoginErrorResponse::AccessDenied.into())
    }
}
