use axum::{
    Router,
    routing::{get, post},
};

use crate::{handlers::fallback::method_not_allowed, state::AppState};

mod dto;
mod handler;
pub mod jobs;
mod repo;
mod services;

pub fn auth_routes() -> Router<AppState> {
    Router::new()
        .route("/email/start", post(handler::email_start))
        .route("/email/verify", post(handler::email_verify))
        .route("/refresh", post(handler::refresh))
        .route("/logout", post(handler::logout))
        .route("/google", get(handler::google))
        .route("/google/callback", get(handler::google_callback))
        .route("/google/exchange", post(handler::google_exchange))
        .method_not_allowed_fallback(method_not_allowed)
}
