use axum::{Json, extract::State, http::StatusCode};
use uuid::Uuid;
use validator::Validate;

use super::{
    dto::{CreatePayoutRequest, PayoutResponse, SaveAsListRequest, SubmitBatchRequest},
    services,
};
use crate::{
    claims::Claims,
    extractors::{AppJson, AppPath},
    handlers::{
        lists::dto::ListDetailResponse,
        payouts::dto::{PreparePayoutRequest, RenamePayoutRequest, UpdatePayoutItemsRequest},
    },
    response::{ApiResponse, ApiResult, ApiResultWithStatus},
    state::AppState,
};

pub async fn create_payout(
    claims: Claims,
    State(state): State<AppState>,
    AppJson(body): AppJson<CreatePayoutRequest>,
) -> ApiResultWithStatus<PayoutResponse> {
    body.validate()?;

    let payout = services::create_payout(&state, claims.sub, body).await?;

    Ok((
        StatusCode::CREATED,
        Json(ApiResponse::new(Some(payout), "Payout created")),
    ))
}

pub async fn get_payout(
    claims: Claims,
    State(state): State<AppState>,
    AppPath(id): AppPath<Uuid>,
) -> ApiResult<PayoutResponse> {
    let payout = services::get_payout(&state, claims.sub, id).await?;

    Ok(Json(ApiResponse::new(Some(payout), "Payout fetched")))
}

pub async fn update_payout_items(
    claims: Claims,
    State(state): State<AppState>,
    AppPath(id): AppPath<Uuid>,
    AppJson(body): AppJson<UpdatePayoutItemsRequest>,
) -> ApiResult<PayoutResponse> {
    body.validate()?;

    let payout = services::update_payout_items(&state, claims.sub, id, body).await?;

    Ok(Json(ApiResponse::new(Some(payout), "Payout updated")))
}

pub async fn rename_payout(
    claims: Claims,
    State(state): State<AppState>,
    AppPath(id): AppPath<Uuid>,
    AppJson(body): AppJson<RenamePayoutRequest>,
) -> ApiResult<PayoutResponse> {
    body.validate()?;

    let payout = services::rename_payout(&state, claims.sub, id, body).await?;

    Ok(Json(ApiResponse::new(Some(payout), "Payout renamed")))
}

pub async fn prepare_payout(
    claims: Claims,
    State(state): State<AppState>,
    AppPath(id): AppPath<Uuid>,
    AppJson(body): AppJson<PreparePayoutRequest>,
) -> ApiResult<PayoutResponse> {
    let payout = services::prepare_payout(&state, claims.sub, id, body).await?;

    Ok(Json(ApiResponse::new(Some(payout), "Payout ready to send")))
}

pub async fn save_as_list(
    claims: Claims,
    State(state): State<AppState>,
    AppPath(id): AppPath<Uuid>,
    AppJson(body): AppJson<SaveAsListRequest>,
) -> ApiResultWithStatus<ListDetailResponse> {
    body.validate()?;

    let list = services::save_as_list(&state, claims.sub, id, body).await?;

    Ok((
        StatusCode::CREATED,
        Json(ApiResponse::new(Some(list), "Saved as a list")),
    ))
}

pub async fn submit_batch(
    claims: Claims,
    State(state): State<AppState>,
    AppPath((id, batch_id)): AppPath<(Uuid, Uuid)>,
    AppJson(body): AppJson<SubmitBatchRequest>,
) -> ApiResult<PayoutResponse> {
    let payout = services::submit_batch(&state, claims.sub, id, batch_id, body).await?;

    Ok(Json(ApiResponse::new(Some(payout), "Transaction recorded")))
}
