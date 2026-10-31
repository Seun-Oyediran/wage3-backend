//! Reference feature. Copy this folder when adding a new resource.
//!
//! mod.rs      -> routes only
//! handler.rs  -> HTTP layer: extract, validate, call service, wrap response
//! services.rs -> business rules; knows nothing about HTTP
//! repo.rs     -> SQL only; returns row structs
//! dto.rs      -> request/response shapes the client sees

use axum::{Router, routing::get};

use crate::{handlers::fallback::method_not_allowed, state::AppState};

mod dto;
mod handler;
mod repo;
mod services;

pub fn template_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(handler::list).post(handler::create))
        .route(
            "/{id}",
            get(handler::get_one)
                .patch(handler::update)
                .delete(handler::delete),
        )
        .method_not_allowed_fallback(method_not_allowed)
}
