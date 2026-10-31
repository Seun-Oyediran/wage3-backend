use axum::{Json, extract::State};

use super::dto::{PaymentsStatus, StatusResponse};
use crate::{
    feature_flags::{self, PAYMENTS_PAUSED},
    response::{ApiResponse, ApiResult},
    state::AppState,
};

pub async fn get_status(State(state): State<AppState>) -> ApiResult<StatusResponse> {
    let payments = feature_flags::get_flag(&state.db, "payments").await?;

    let message = (!payments.enabled).then(|| {
        payments
            .reason
            .unwrap_or_else(|| PAYMENTS_PAUSED.to_string())
    });

    Ok(Json(ApiResponse::new(
        Some(StatusResponse {
            payments: PaymentsStatus {
                enabled: payments.enabled,
                message,
            },
        }),
        "Successful",
    )))
}
