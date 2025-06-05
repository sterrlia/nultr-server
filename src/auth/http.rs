use axum::{
    Json, RequestPartsExt,
    body::Body,
    extract::FromRequestParts,
    http::{Response, StatusCode, request},
    response::IntoResponse,
};
use axum_extra::TypedHeader;
use headers::HeaderMapExt;
use headers::{Authorization, authorization::Bearer};
use serde::Serialize;

use crate::state;

use super::jwt;

#[derive(Serialize, Debug)]
#[serde(tag = "code")]
pub enum AuthError {
    InvalidToken,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response<Body> {
        match self {
            AuthError::InvalidToken => (StatusCode::BAD_REQUEST, self).into_response(),
        }
    }
}

impl FromRequestParts<state::ServiceState> for jwt::Claims {
    type Rejection = AuthError;

    async fn from_request_parts(
        parts: &mut request::Parts,
        state: &state::ServiceState,
    ) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| AuthError::InvalidToken)?;

        let token = bearer.token();

        let token_data = state
            .jwt_encoder
            .decode(token.to_string())
            .map_err(|_| AuthError::InvalidToken)?;

        Ok(token_data)
    }
}
