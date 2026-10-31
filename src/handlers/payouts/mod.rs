use axum::{
    Router,
    routing::{get, post, put},
};

use crate::{handlers::fallback::method_not_allowed, state::AppState};

mod dto;
mod handler;
mod repo;
mod services;

pub fn payouts_routes() -> Router<AppState> {
    Router::new()
        .route("/", post(handler::create_payout))
        .route(
            "/{id}",
            get(handler::get_payout).patch(handler::rename_payout),
        )
        .route("/{id}/items", put(handler::update_payout_items))
        .route("/{id}/prepare", post(handler::prepare_payout))
        .route("/{id}/save-as-list", post(handler::save_as_list))
        .route(
            "/{id}/batches/{batch_id}/submitted",
            post(handler::submit_batch),
        )
        .method_not_allowed_fallback(method_not_allowed)
}
