use axum::{
    Json,
    extract::{self, Query},
};
use nultr_shared_lib::{
    request::{
        AuthenticatedUnexpectedErrorResponse, CreateRoomErrorResponse, CreateRoomRequest,
        CreateRoomResponse, GetMessagesErrorResponse, GetMessagesRequest, GetMessagesResponse,
        GetRoomsErrorResponse, GetRoomsRequest, GetRoomsResponse, GetUsersErrorResponse,
        GetUsersResponse, LoginErrorResponse, LoginRequest, LoginResponse, MessageResponse,
        RoomResponse, UnexpectedErrorResponse, UserResponse,
    },
    util::MonoResult,
};
use rust_api_kit::http::client::{Response};
use sea_orm::ActiveValue::Set;

use crate::{
    auth,
    db::{
        self, RepositoryTrait,
        entity::rooms::{self, ActiveModel},
    },
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

pub async fn get_rooms(
    extract::State(state): extract::State<state::ServiceState>,
    claims: auth::jwt::Claims,
) -> AuthenticatedResponse<GetRoomsResponse, GetRoomsErrorResponse> {
    let rooms = state
        .room_repository
        .get_by_user_id(claims.user_id)
        .await?
        .iter()
        .map(|room| RoomResponse {
            id: room.id,
            name: room.name.clone(),
        })
        .collect();

    Ok(Response::Ok(GetRoomsResponse(rooms)))
}

pub async fn create_room(
    extract::State(state): extract::State<state::ServiceState>,
    claims: auth::jwt::Claims,
    Json(input): Json<CreateRoomRequest>,
) -> AuthenticatedResponse<CreateRoomResponse, CreateRoomErrorResponse> {
    let room = state
        .room_repository
        .insert(rooms::ActiveModel {
            name: Set(input.name),
            ..Default::default()
        })
        .await?;

    state
        .room_repository
        .add_users_to_room(room.id, vec![claims.user_id, input.receiver_user_id])
        .await?;

    Ok(Response::Ok(CreateRoomResponse {
        id: room.id,
        name: room.name,
    }))
}

pub async fn get_messages(
    Query(request): Query<GetMessagesRequest>,
    extract::State(state): extract::State<state::ServiceState>,
    claims: auth::jwt::Claims,
) -> AuthenticatedResponse<GetMessagesResponse, GetMessagesErrorResponse> {
    let room_exists = state.room_repository.exists_by_id(request.room_id).await?;
    if !room_exists {
        return Err(GetMessagesErrorResponse::RoomNotFound.into());
    }

    let room_users = state
        .room_repository
        .get_users_by_room(request.room_id)
        .await?;

    let is_member_of_room = room_users.iter().any(|user| user.id == claims.user_id);
    if !is_member_of_room {
        return Err(GetMessagesErrorResponse::NotMemberOfRoom.into());
    }

    let messages = state
        .message_repository
        .get_messages_by_room(
            request.room_id,
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
                uuid: message.uuid,
                user_id: message.user_id,
                content: message.content.clone(),
                created_at: message.created_at,
                read: message.read,
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
