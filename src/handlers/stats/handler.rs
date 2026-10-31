use axum::{Json, extract::State};

use super::{
    dto::{AdminStatsQuery, AdminStatsResponse, AppStatsResponse},
    services,
};
use crate::{
    claims::Claims,
    extractors::AppQuery,
    response::{ApiResponse, ApiResult},
    state::AppState,
};

pub async fn get_app_stats(State(state): State<AppState>) -> ApiResult<AppStatsResponse> {
    let stats = services::get_app_stats(&state).await?;

    Ok(Json(ApiResponse::new(Some(stats), "Stats fetched")))
}

pub async fn get_admin_stats(
    _claims: Claims,
    State(state): State<AppState>,
    AppQuery(query): AppQuery<AdminStatsQuery>,
) -> ApiResult<AdminStatsResponse> {
    let stats = services::get_admin_stats(&state, query).await?;

    Ok(Json(ApiResponse::new(Some(stats), "Stats fetched")))
}
