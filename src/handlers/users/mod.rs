use axum::{Router, routing::get};

use crate::{handlers::fallback::method_not_allowed, state::AppState};

pub mod dto;
mod handler;
pub mod repo;
mod services;

pub fn users_routes() -> Router<AppState> {
    Router::new()
        .route("/me", get(handler::get_me))
        .route("/me/stats", get(handler::get_stats))
        .method_not_allowed_fallback(method_not_allowed)
}
