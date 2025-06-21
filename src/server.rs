use axum::{
    Router,
    extract::{self, Query},
    routing::{any, post},
};
use nultr_shared_lib::request::{GetMessagesRequest, GetUsersRequest, LoginRequest};
use rust_api_integrator::generate_routes;
use tokio::sync::Mutex;

use std::{collections::HashMap, net::SocketAddr, path::PathBuf, sync::Arc};
use tower_http::{
    services::ServeDir,
    trace::{DefaultMakeSpan, TraceLayer},
};

use axum::extract::connect_info::ConnectInfo;

use crate::{auth, config, http, state, ws};

pub async fn serve() {
    let assets_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");

    let http_api_routes = generate_routes! {
        LoginRequest => http::controller::login,
        GetUsersRequest => http::controller::get_users,
        GetMessagesRequest => http::controller::get_messages,
    };

    let ws_state = Arc::new(Mutex::new(state::MutexState {
        user_message_sender_map: HashMap::new(),
    }));

    // build our application with some routes
    let app = Router::new()
        .fallback_service(ServeDir::new(assets_dir).append_index_html_on_directories(true))
        .merge(http_api_routes)
        .route(
            "/ws",
            any({
                move |ws: axum::extract::WebSocketUpgrade,
                ConnectInfo(addr): ConnectInfo<SocketAddr>,
                extract::State(state): extract::State<state::ServiceState>,
                claims: auth::jwt::Claims| {
                    ws::connector::handle(ws, addr, state, claims, ws_state.clone())
                }
            }),
        )
        .with_state(state::ServiceState::default())
        .layer(
            TraceLayer::new_for_http()
            .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        );

    // run it with hyper
    let listener = tokio::net::TcpListener::bind(config::WS_URL.clone())
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
