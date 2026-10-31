use axum::{Json, extract::State, http::StatusCode};
use uuid::Uuid;
use validator::Validate;

use super::{
    dto::{
        AddRecipientsRequest, CreateListRequest, ListDetailResponse, ListResponse,
        RenameListRequest, ReplaceRecipientsRequest,
    },
    services,
};
use crate::{
    claims::Claims,
    extractors::{AppJson, AppPath, AppQuery},
    handlers::lists::dto::GetListRequestQuery,
    response::{ApiResponse, ApiResult, ApiResultWithStatus},
    state::AppState,
};

pub async fn create_list(
    claims: Claims,
    State(state): State<AppState>,
    AppJson(body): AppJson<CreateListRequest>,
) -> ApiResultWithStatus<ListDetailResponse> {
    body.validate()?;

    let res = services::create_list(&state, claims.sub, body).await?;

    Ok((
        StatusCode::CREATED,
        Json(ApiResponse::new(Some(res), "Created successfully")),
    ))
}

pub async fn get_lists(
    claims: Claims,
    State(state): State<AppState>,
    AppQuery(query): AppQuery<GetListRequestQuery>,
) -> ApiResult<Vec<ListResponse>> {
    let res = services::get_lists(&state, claims.sub, query.search.as_deref()).await?;

    Ok(Json(ApiResponse::new(Some(res), "Successful")))
}

pub async fn get_list(
    claims: Claims,
    State(state): State<AppState>,
    AppPath(id): AppPath<Uuid>,
) -> ApiResult<ListDetailResponse> {
    let res = services::get_list(&state, claims.sub, id).await?;

    Ok(Json(ApiResponse::new(Some(res), "List fetched")))
}

pub async fn rename_list(
    claims: Claims,
    State(state): State<AppState>,
    AppPath(id): AppPath<Uuid>,
    AppJson(body): AppJson<RenameListRequest>,
) -> ApiResult<ListDetailResponse> {
    body.validate()?;

    let res = services::rename_list(&state, claims.sub, id, body).await?;

    Ok(Json(ApiResponse::new(Some(res), "List renamed")))
}

pub async fn delete_list(
    claims: Claims,
    State(state): State<AppState>,
    AppPath(id): AppPath<Uuid>,
) -> ApiResult<()> {
    services::delete_list(&state, claims.sub, id).await?;

    Ok(Json(ApiResponse::new(None, "List deleted")))
}

pub async fn add_recipients(
    claims: Claims,
    State(state): State<AppState>,
    AppPath(id): AppPath<Uuid>,
    AppJson(body): AppJson<AddRecipientsRequest>,
) -> ApiResult<ListDetailResponse> {
    body.validate()?;

    let res = services::add_recipients(&state, claims.sub, id, body).await?;

    Ok(Json(ApiResponse::new(Some(res), "Recipients added")))
}

pub async fn replace_recipients(
    claims: Claims,
    State(state): State<AppState>,
    AppPath(id): AppPath<Uuid>,
    AppJson(body): AppJson<ReplaceRecipientsRequest>,
) -> ApiResult<ListDetailResponse> {
    body.validate()?;

    let res = services::replace_recipients(&state, claims.sub, id, body).await?;

    Ok(Json(ApiResponse::new(Some(res), "Recipients updated")))
}
