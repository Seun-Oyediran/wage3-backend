use std::sync::Arc;

use jsonwebtoken::{DecodingKey, EncodingKey};
use sqlx::PgPool;

use crate::{config::Config, google::GoogleOidc};

#[derive(Debug, Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: PgPool,
    pub google: GoogleOidc,
    pub jwt_encoded_secret: EncodingKey,
    pub jwt_refresh_encoded_secret: EncodingKey,
    pub jwt_decoded_secret: DecodingKey,
    pub jwt_refresh_decoded_secret: DecodingKey,
}

impl AppState {
    pub fn new(config: Config, db: PgPool, google: GoogleOidc) -> Self {
        let jwt_encoded_secret = EncodingKey::from_secret(config.jwt_secret.as_ref());
        let jwt_refresh_encoded_secret =
            EncodingKey::from_secret(config.jwt_refresh_secret.as_ref());

        let jwt_decoded_secret = DecodingKey::from_secret(config.jwt_secret.as_ref());
        let jwt_refresh_decoded_secret =
            DecodingKey::from_secret(config.jwt_refresh_secret.as_ref());

        Self {
            config: Arc::new(config),
            db,
            google,
            jwt_encoded_secret,
            jwt_refresh_encoded_secret,
            jwt_decoded_secret,
            jwt_refresh_decoded_secret,
        }
    }
}
