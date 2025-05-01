//! Example websocket server.
//!
//! Run the server with
//! ```not_rust
//! cargo run -p example-websockets --bin example-websockets
//! ```
//!
//! Run a browser client with
//! ```not_rust
//! firefox http://localhost:3000
//! ```
//!
//! Alternatively you can run the rust client (showing two
//! concurrent websocket connections being established) with
//! ```not_rust
//! cargo run -p example-websockets --bin example-client
//! ```

use axum::{
    Json, Router,
    extract::{
        Query,
        ws::{WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    response::IntoResponse,
    routing::any,
};
use serde_json::json;
use tokio::sync::mpsc::{self, UnboundedReceiver};
use websocket_handler::handler::{MessageFromUser, UserMessageSenderMap};

use std::{collections::HashMap, net::SocketAddr, path::PathBuf, sync::Arc};
use tower_http::{
    services::ServeDir,
    trace::{DefaultMakeSpan, TraceLayer},
};

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

//allows to extract the IP of connecting user
use axum::extract::connect_info::ConnectInfo;

//allows to split the websocket stream into separate TX and RX branches
use futures::{lock::Mutex, stream::StreamExt};

mod websocket_handler;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("{}=debug,tower_http=debug", env!("CARGO_CRATE_NAME")).into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let assets_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");

    let user_map: UserMessageSenderMap = Arc::new(Mutex::new(HashMap::new()));

    // build our application with some routes
    let app = Router::new()
        .fallback_service(ServeDir::new(assets_dir).append_index_html_on_directories(true))
        .route(
            "/ws",
            any({
                move |ws: axum::extract::WebSocketUpgrade,
                      ConnectInfo(addr): ConnectInfo<SocketAddr>,
                      Query(params): Query<HashMap<String, String>>| {
                    ws_handler(ws, addr, params, user_map.clone())
                }
            }),
        ) // logging so we can see what's going on
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        );

    // run it with hyper
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3005")
        .await
        .unwrap();

    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

/// The handler for the HTTP request (this gets called when the HTTP request lands at the start
/// of websocket negotiation). After this completes, the actual switching from HTTP to
/// websocket protocol will occur.
/// This is the last point where we can extract TCP/IP metadata such as IP address of the client
/// as well as things from HTTP headers such as user-agent of the browser etc.
async fn ws_handler(
    ws: WebSocketUpgrade,
    addr: SocketAddr,
    query: HashMap<String, String>,
    user_map: UserMessageSenderMap,
) -> impl IntoResponse {
    println!("{addr} connected.");

    let username = match &query.get("username") {
        Some(u) => u.to_string(),
        None => {
            let body = Json(json!({
                "error": "Forbidden",
                "message": "Invalid or missing token"
            }));
            return (StatusCode::FORBIDDEN, body).into_response();
        }
    };

    let (tx, rx) = mpsc::unbounded_channel::<MessageFromUser>();

    user_map.lock().await.insert(username.clone(), tx);

    // finalize the upgrade process by returning upgrade callback.
    // we can customize the callback by sending additional info such as address.
    ws.on_upgrade(move |socket| handle_socket(socket, addr, user_map, rx, username))
}

/// Actual websocket statemachine (one will be spawned per connection)
async fn handle_socket(
    socket: WebSocket,
    addr: SocketAddr,
    user_message_sender_map: UserMessageSenderMap,
    user_message_receiver: UnboundedReceiver<MessageFromUser>,
    username: String
) {
    let (ws_sender, ws_receiver) = socket.split();

    let mut handler = websocket_handler::Handler {
        username,
        user_message_sender_map,
        user_message_receiver,
        ws_sender,
        ws_receiver,
    };

    handler.listen().await;

    println!("Websocket context {addr} destroyed");
}
