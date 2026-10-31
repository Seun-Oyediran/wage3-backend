use axum::{Router, routing::get};

use crate::{handlers::fallback::method_not_allowed, state::AppState};

mod dto;
mod handler;

/// Public: the frontend shows the banner before login too.
pub fn status_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(handler::get_status))
        .method_not_allowed_fallback(method_not_allowed)
}
