use jsonwebtoken::{EncodingKey, errors::Error as JsonWebTokenError};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{claims::Claims, handlers::users::dto::UserResponse};

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct EmailStartRequest {
    #[validate(email)]
    pub email: String,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct EmailVerifyRequest {
    #[validate(email)]
    pub email: String,
    #[validate(length(equal = 6, message = "OTP is invalid"))]
    pub code: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

/// Access + refresh token pair returned after a successful sign-in or refresh.
#[derive(Debug, Serialize)]
pub struct AuthTokensResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub access_expires_at: usize,
    pub refresh_expires_at: usize,
}

impl AuthTokensResponse {
    pub fn new(
        access_claims: &Claims,
        refresh_claims: &Claims,
        access_key: &EncodingKey,
        refresh_key: &EncodingKey,
    ) -> Result<Self, JsonWebTokenError> {
        Ok(Self {
            access_token: access_claims.encode_jwt(access_key)?,
            refresh_token: refresh_claims.encode_jwt(refresh_key)?,
            access_expires_at: access_claims.exp,
            refresh_expires_at: refresh_claims.exp,
        })
    }
}

/// Returned after a successful sign-in: who the user is plus their tokens.
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub user: UserResponse,
    pub tokens: AuthTokensResponse,
}

/// Query string Google sends to the callback. Either `code` + `state`, or `error`.
/// No `deny_unknown_fields`: Google also sends `scope`, `authuser`, `prompt`, etc.
#[derive(Debug, Deserialize)]
pub struct GoogleCallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
}

/// The one-time ticket from the Google callback redirect.
#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct GoogleExchangeRequest {
    #[validate(length(equal = 64, message = "Invalid ticket"))]
    pub ticket: String,
}
