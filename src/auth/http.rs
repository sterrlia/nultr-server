use axum::{
    Json, RequestPartsExt,
    body::Body,
    extract::FromRequestParts,
    http::{Response, StatusCode, header, request},
    response::IntoResponse,
};
use headers::{Authorization, authorization::Bearer};
use serde::Serialize;

use crate::state;

use super::jwt;

#[derive(Serialize, Debug)]
#[serde(tag = "type")]
pub enum AuthError {
    InvalidToken,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response<Body> {
        let body = Json(&self);

        match self {
            AuthError::InvalidToken => (StatusCode::BAD_REQUEST, body).into_response(),
        }
    }
}

impl FromRequestParts<state::ServiceState> for jwt::Claims {
    type Rejection = AuthError;

    async fn from_request_parts(
        parts: &mut request::Parts,
        state: &state::ServiceState,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(header::AUTHORIZATION)
            .ok_or({
                tracing::error!("Missing auth header");
                AuthError::InvalidToken
            })?
            .to_str()
            .map_err(|err| {
                tracing::error!("Cannot convert auth header to str {err}");

                AuthError::InvalidToken
            })?
            .trim_start_matches(|c: char| c.is_whitespace() || c.is_control());

        let token = auth_header.strip_prefix("Bearer ").ok_or({
            tracing::error!("Cannot strip bearer prefix on {auth_header}");

            AuthError::InvalidToken
        })?;

        let token_data = state
            .jwt_encoder
            .decode(token.to_string())
            .map_err(|_| AuthError::InvalidToken)?;

        Ok(token_data)
    }
}
