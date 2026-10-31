use axum::{Router, routing::get};

use crate::{handlers::fallback::method_not_allowed, state::AppState};

mod dto;
mod handler;
mod repo;
mod services;

/// Public totals for the landing page. To enable: `.nest("/stats", stats_routes())`.
pub fn stats_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(handler::get_app_stats))
        .method_not_allowed_fallback(method_not_allowed)
}

/// Internal numbers. Needs an admin-only guard before it's mounted:
/// right now any logged-in user would pass `Claims`.
pub fn admin_stats_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(handler::get_admin_stats))
        .method_not_allowed_fallback(method_not_allowed)
}
