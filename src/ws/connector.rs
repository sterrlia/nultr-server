use axum::{
    extract::ws::{WebSocket, WebSocketUpgrade},
    response::IntoResponse,
};
use tokio::sync::{
    Mutex,
    mpsc::{self, UnboundedReceiver},
};

use std::{net::SocketAddr, sync::Arc};

use futures::stream::StreamExt;

use crate::{
    auth,
    state::{self, ThreadEvent},
};

use super::controller;

pub async fn handle(
    ws: WebSocketUpgrade,
    addr: SocketAddr,
    service_state: state::ServiceState,
    claims: auth::jwt::Claims,
    mutex_state: Arc<Mutex<state::MutexState>>,
) -> impl IntoResponse {
    tracing::debug!("{addr} connected.");

    let (tx, rx) = mpsc::unbounded_channel::<ThreadEvent>();

    mutex_state
        .lock()
        .await
        .user_message_sender_map
        .insert(claims.user_id, tx);

    ws.on_upgrade(move |socket| handle_socket(socket, addr, service_state, mutex_state, claims, rx))
}

async fn handle_socket(
    socket: WebSocket,
    addr: SocketAddr,
    service_state: state::ServiceState,
    mutex_state: Arc<Mutex<state::MutexState>>,
    claims: auth::jwt::Claims,
    user_message_receiver: UnboundedReceiver<ThreadEvent>,
) {
    let user_id = claims.user_id;

    let (ws_sender, ws_receiver) = socket.split();

    let mut handler = controller::Controller {
        mutex_state: mutex_state.clone(),
        service_state,
        claims,
        user_message_receiver,
        ws_sender,
        ws_receiver,
    };

    tracing::debug!("Websocket handler started {addr}");

    while let Some(msg) = handler.get_message().await {
        if let Err(error) = handler.process(msg).await {
            tracing::error!("Websocket handle message error: {error}");
            break;
        }
    }

    tracing::debug!("Websocket context {addr} destroyed");

    mutex_state
        .lock()
        .await
        .user_message_sender_map
        .remove(&user_id);
}
