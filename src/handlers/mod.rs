use axum::Router;

use crate::{
    handlers::{
        auth::auth_routes, history::history_routes, lists::lists_routes, payouts::payouts_routes,
        status::status_routes, template::template_routes, users::users_routes,
    },
    state::AppState,
};

pub mod auth;
pub mod fallback;
mod history;
mod lists;
mod payouts;
#[allow(dead_code)] // built but not mounted yet, see stats/mod.rs
mod stats;
mod status;
mod template;
mod users;

pub fn v1_routes() -> Router<AppState> {
    Router::new()
        .nest("/template", template_routes())
        .nest("/auth", auth_routes())
        .nest("/users", users_routes())
        .nest("/lists", lists_routes())
        .nest("/payouts", payouts_routes())
        .nest("/history", history_routes())
        .nest("/status", status_routes())
}

pub fn app_routes() -> Router<AppState> {
    Router::new().nest("/v1", v1_routes())
}
