use axum::extract::FromRequestParts;
use chrono::{TimeDelta, Utc};
use jsonwebtoken::{
    Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation,
    errors::Error as JsonWebTokenError,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{error::AppError, state::AppState};

/// JWT payload for both access and refresh tokens.
/// Also an extractor: add `claims: Claims` to a handler to require a valid access token.
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub email: String,
    pub exp: usize,
    pub iat: usize,
    pub jti: Uuid,
}

impl Claims {
    pub fn new(sub: Uuid, email: String, duration: TimeDelta) -> Self {
        let now = Utc::now();
        let exp = (now + duration).timestamp() as usize;
        let iat = now.timestamp() as usize;

        Self {
            sub,
            email,
            exp,
            iat,
            jti: Uuid::new_v4(),
        }
    }

    pub fn encode_jwt(&self, key: &EncodingKey) -> Result<String, JsonWebTokenError> {
        jsonwebtoken::encode(&Header::default(), self, key)
    }

    pub fn decode_jwt(
        token: &str,
        key: &DecodingKey,
    ) -> Result<TokenData<Claims>, JsonWebTokenError> {
        jsonwebtoken::decode::<Claims>(token, key, &Validation::new(Algorithm::HS256))
    }
}

impl FromRequestParts<AppState> for Claims {
    type Rejection = AppError;
    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let error_message = String::from("Unauthorized");

        let (prefix, token) = parts
            .headers
            .get("Authorization")
            .ok_or_else(|| AppError::Unauthorized(error_message.clone()))?
            .to_str()
            .map_err(|_| AppError::Unauthorized(error_message.clone()))?
            .split_once(' ')
            .ok_or_else(|| AppError::Unauthorized(error_message.clone()))?;

        if !prefix.eq_ignore_ascii_case("Bearer") {
            return Err(AppError::Unauthorized(error_message.clone()));
        }

        let claims = Claims::decode_jwt(token, &state.jwt_decoded_secret)
            .map_err(|_| AppError::Unauthorized(error_message))?;

        Ok(claims.claims)
    }
}
