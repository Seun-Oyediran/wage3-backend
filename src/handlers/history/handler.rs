use axum::{Json, extract::State};
use validator::Validate;

use super::{
    dto::{SentHistoryQuery, SentPayoutResponse},
    services,
};
use crate::{
    claims::Claims,
    extractors::AppQuery,
    response::{ApiResponse, ApiResult, PaginatedList, PaginatedRequest},
    state::AppState,
};

pub async fn get_sent(
    claims: Claims,
    State(state): State<AppState>,
    AppQuery(query): AppQuery<SentHistoryQuery>,
    AppQuery(paging): AppQuery<PaginatedRequest>,
) -> ApiResult<PaginatedList<SentPayoutResponse>> {
    paging.validate()?;

    let res = services::get_sent(&state, claims.sub, query, &paging).await?;

    Ok(Json(ApiResponse::new(Some(res), "Sent history fetched")))
}
