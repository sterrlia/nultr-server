use axum::{
    extract::FromRequestParts,
    http::{header, request},
};
use nultr_shared_lib::request::AuthError;
use rust_api_kit::http::client::Response;

use crate::state;

use super::jwt;

impl FromRequestParts<state::ServiceState> for jwt::Claims {
    type Rejection = Response<(), (), AuthError>;

    async fn from_request_parts(
        parts: &mut request::Parts,
        state: &state::ServiceState,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(header::AUTHORIZATION)
            .ok_or({
                tracing::error!("Missing auth header");

                Response::UnexpectedError(AuthError::InvalidToken)
            })?
            .to_str()
            .map_err(|err| {
                tracing::error!("Cannot convert auth header to str {err}");

                Response::UnexpectedError(AuthError::InvalidToken)
            })?
            .trim_start_matches(|c: char| c.is_whitespace() || c.is_control());

        let token = auth_header
            .strip_prefix("Bearer ")
            .or_else(|| auth_header.strip_prefix("bearer "))
            .ok_or({
                tracing::error!("Cannot strip bearer prefix on {auth_header}");

                Response::UnexpectedError(AuthError::InvalidToken)
            })?;

        let token_data = state
            .jwt_encoder
            .decode(token.to_string())
            .inspect_err(|err| {
                tracing::error!(
                    "Bearer decode failed for token: {token}, with error: {:?}",
                    err
                )
            })
            .map_err(|_| Response::UnexpectedError(AuthError::InvalidToken))?;

        Ok(token_data)
    }
}
