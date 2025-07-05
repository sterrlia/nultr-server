use axum::{
    Json,
    extract::{self, Query},
};
use nultr_shared_lib::{
    request::{
        AuthenticatedUnexpectedErrorResponse, CreatePrivateRoomErrorResponse,
        CreatePrivateRoomRequest, CreatePrivateRoomResponse, GetMessagesErrorResponse,
        GetMessagesRequest, GetMessagesResponse, GetRoomsErrorResponse,
        GetRoomsResponse, GetUsersErrorResponse, GetUsersResponse, LoginErrorResponse,
        LoginRequest, LoginResponse, MessageResponse, RoomResponse, UnexpectedErrorResponse,
        UserResponse,
    },
    util::MonoResult,
};
use rust_api_kit::http::client::Response;
use sea_orm::ActiveValue::Set;

use crate::{
    auth,
    db::{
        self, RepositoryTrait,
        entity::{
            rooms::{self},
            rooms_users,
        },
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
        .get_for_user(claims.user_id)
        .await?
        .iter()
        .map(|room| RoomResponse {
            id: room.id,
            name: room.name.clone(),
        })
        .collect();

    Ok(Response::Ok(GetRoomsResponse(rooms)))
}

pub async fn create_private_room(
    extract::State(state): extract::State<state::ServiceState>,
    claims: auth::jwt::Claims,
    Json(input): Json<CreatePrivateRoomRequest>,
) -> AuthenticatedResponse<CreatePrivateRoomResponse, CreatePrivateRoomErrorResponse> {
    //let txn = state.room_repository.begin_transaction().await?;
    // TODO: validate, lock by current user

    let (recipient_result, current_user_result) = tokio::join!(
        state.user_repository.get_by_id(input.receiver_user_id),
        state.user_repository.get_by_id(claims.user_id)
    );

    let recipient = recipient_result?.ok_or(Response::Error(
        CreatePrivateRoomErrorResponse::UserNotFound,
    ))?;

    let current_user = current_user_result?.ok_or(Response::Error(
        CreatePrivateRoomErrorResponse::UserNotFound,
    ))?;

    let room = state
        .room_repository
        .insert(rooms::ActiveModel {
            name: Set(input.name),
            ..Default::default()
        })
        .await?;

    let room_name_for_current_usr = recipient.username;

    let current_user_link = rooms_users::ActiveModel {
        room_id: Set(room.id),
        user_id: Set(claims.user_id),
        generated_room_name: Set(Some(room_name_for_current_usr.clone())),
    };

    let recipient_link = rooms_users::ActiveModel {
        room_id: Set(room.id),
        user_id: Set(recipient.id),
        generated_room_name: Set(Some(current_user.username)),
    };

    state
        .room_repository
        .insert_rooms_users(vec![current_user_link, recipient_link])
        .await?;

    //state.room_repository.end_transaction(txn).await?;

    Ok(Response::Ok(CreatePrivateRoomResponse {
        id: room.id,
        name: room_name_for_current_usr,
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
