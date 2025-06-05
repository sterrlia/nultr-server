use axum::{
    Json, extract::
        ws::{WebSocket, WebSocketUpgrade}
    ,
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::json;
use tokio::sync::{
    Mutex,
    mpsc::{self, UnboundedReceiver},
};

use std::{collections::HashMap, net::SocketAddr, sync::Arc};

//allows to split the websocket stream into separate TX and RX branches
use futures::stream::StreamExt;

use crate::{auth, state::{self, MessageFromUser}};

use super::controller;

/// The handler for the HTTP request (this gets called when the HTTP request lands at the start
/// of websocket negotiation). After this completes, the actual switching from HTTP to
/// websocket protocol will occur.
/// This is the last point where we can extract TCP/IP metadata such as IP address of the client
/// as well as things from HTTP headers such as user-agent of the browser etc.
pub async fn handle(
    ws: WebSocketUpgrade,
    addr: SocketAddr,
    service_state: state::ServiceState,
    claims: auth::jwt::Claims,
    mutex_state: Arc<Mutex<state::MutexState>>,
) -> impl IntoResponse {
    tracing::debug!("{addr} connected.");

    let (tx, rx) = mpsc::unbounded_channel::<MessageFromUser>();

    mutex_state
        .lock()
        .await
        .user_message_sender_map
        .insert(claims.user_id, tx);

    // finalize the upgrade process by returning upgrade callback.
    // we can customize the callback by sending additional info such as address.
    ws.on_upgrade(move |socket| {
        handle_socket(
            socket,
            addr,
            service_state,
            mutex_state,
            claims,
            rx,
        )
    })
}

/// Actual websocket statemachine (one will be spawned per connection)
async fn handle_socket(
    socket: WebSocket,
    addr: SocketAddr,
    service_state: state::ServiceState,
    mutex_state: Arc<Mutex<state::MutexState>>,
    claims: auth::jwt::Claims,
    user_message_receiver: UnboundedReceiver<MessageFromUser>
) {
    let (ws_sender, ws_receiver) = socket.split();

    let mut handler = controller::Controller {
        mutex_state,
        service_state,
        claims,
        user_message_receiver,
        ws_sender,
        ws_receiver,
    };

    while let Some(msg) = handler.get_message().await {
        if let Err(error) = handler.process(msg).await {
            tracing::error!("Websocket handle message error: {error}");
            break;
        }
    }

    tracing::debug!("Websocket context {addr} destroyed");
}
