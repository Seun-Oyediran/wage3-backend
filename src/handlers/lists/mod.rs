use axum::{
    Router,
    routing::{get, post},
};

use crate::{handlers::fallback::method_not_allowed, state::AppState};

pub mod dto;
mod handler;
pub mod repo;
mod services;

pub fn lists_routes() -> Router<AppState> {
    Router::new()
        .route("/", post(handler::create_list).get(handler::get_lists))
        .route(
            "/{id}",
            get(handler::get_list)
                .patch(handler::rename_list)
                .delete(handler::delete_list),
        )
        .route(
            "/{id}/recipients",
            post(handler::add_recipients).put(handler::replace_recipients),
        )
        .method_not_allowed_fallback(method_not_allowed)
}
