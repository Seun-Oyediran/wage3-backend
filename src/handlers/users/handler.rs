use axum::{Json, extract::State};

use super::{
    dto::{StatsQuery, StatsResponse, UserResponse},
    services,
};
use crate::{
    claims::Claims,
    extractors::AppQuery,
    response::{ApiResponse, ApiResult},
    state::AppState,
};

pub async fn get_me(claims: Claims, State(state): State<AppState>) -> ApiResult<UserResponse> {
    let user = services::get_me(&state, claims.sub).await?;

    Ok(Json(ApiResponse::new(Some(user), "User fetched")))
}

pub async fn get_stats(
    claims: Claims,
    State(state): State<AppState>,
    AppQuery(query): AppQuery<StatsQuery>,
) -> ApiResult<StatsResponse> {
    let stats = services::get_stats(&state, claims.sub, query).await?;

    Ok(Json(ApiResponse::new(Some(stats), "Stats fetched")))
}
