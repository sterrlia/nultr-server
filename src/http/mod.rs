use axum::{body::Body, http::{Response, StatusCode}, response::IntoResponse, Json};
use serde::Serialize;

pub mod controller;

#[derive(Serialize)]
#[serde(tag = "code")]
pub enum ErrorResponse {
    InternalServerError,
    UserNotFound,
    AccessDenied
}

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> Response<Body> {
        match self {
            ErrorResponse::InternalServerError => {
                        (StatusCode::INTERNAL_SERVER_ERROR, Json(self)).into_response()
                    }
            ErrorResponse::UserNotFound => (StatusCode::NOT_FOUND, Json(self)).into_response(),
            ErrorResponse::AccessDenied => (StatusCode::FORBIDDEN, Json(self)).into_response(),
        }
    }
}

impl From<anyhow::Error> for ErrorResponse {
    fn from(value: anyhow::Error) -> Self {
        tracing::error!("{}", value.to_string());

        ErrorResponse::InternalServerError
    }
}

