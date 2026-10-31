use axum::{Router, routing::get};

use crate::{handlers::fallback::method_not_allowed, state::AppState};

mod dto;
mod handler;
mod repo;
mod services;

pub fn history_routes() -> Router<AppState> {
    Router::new()
        .route("/sent", get(handler::get_sent))
        .method_not_allowed_fallback(method_not_allowed)
}
