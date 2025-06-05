use anyhow::anyhow;
use axum::{
    RequestExt,
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
    response::IntoResponse,
};
use jsonwebtoken::{DecodingKey, Validation, decode, errors::Error as JwtError};
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::config;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub user_id: i32,
    pub exp: usize,
}

#[derive(Clone)]
pub struct Encoder {
    secret: String,
}

impl Default for Encoder {
    fn default() -> Self {
        Self {
            secret: config::JWT_SECRET_KEY.clone(),
        }
    }
}
impl Encoder {
    pub fn encode(&self, user_id: i32) -> Result<String, anyhow::Error> {
        let expiration = SystemTime::now()
            .checked_add(Duration::from_secs(60 * 60))
            .unwrap()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;

        let claims = Claims {
            user_id,
            exp: expiration,
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_ref()),
        )
        .map_err(|err| anyhow!(err))
    }

    pub fn decode(&self, token: String) -> Result<Claims, JwtError> {
        let token_data = decode::<Claims>(
            token.as_str(),
            &DecodingKey::from_secret(self.secret.as_ref()),
            &Validation::default(),
        )?;
        Ok(token_data.claims)
    }
}


