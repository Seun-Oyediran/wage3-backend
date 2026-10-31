//! HTTP layer: extract input, validate it, call the service, pick the status code.
//! Keep these thin: no SQL, no business rules.

use axum::{Json, extract::State, http::StatusCode};
use uuid::Uuid;
use validator::Validate;

use super::{
    dto::{CreateTemplateRequest, TemplateResponse, UpdateTemplateRequest},
    services,
};
use crate::{
    extractors::{AppJson, AppPath, AppQuery},
    response::{ApiResponse, ApiResult, ApiResultWithStatus, PaginatedList, PaginatedRequest},
    state::AppState,
};

pub async fn create(
    State(state): State<AppState>,
    AppJson(body): AppJson<CreateTemplateRequest>,
) -> ApiResultWithStatus<TemplateResponse> {
    body.validate()?;

    let template = services::create(&state, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(ApiResponse::new(Some(template), "Template created")),
    ))
}

pub async fn list(
    State(state): State<AppState>,
    AppQuery(query): AppQuery<PaginatedRequest>,
) -> ApiResult<PaginatedList<TemplateResponse>> {
    query.validate()?;

    let templates = services::list(&state, &query).await?;
    Ok(Json(ApiResponse::new(Some(templates), "Templates fetched")))
}

pub async fn get_one(
    State(state): State<AppState>,
    AppPath(id): AppPath<Uuid>,
) -> ApiResult<TemplateResponse> {
    let template = services::get(&state, id).await?;
    Ok(Json(ApiResponse::new(Some(template), "Template fetched")))
}

pub async fn update(
    State(state): State<AppState>,
    AppPath(id): AppPath<Uuid>,
    AppJson(body): AppJson<UpdateTemplateRequest>,
) -> ApiResult<TemplateResponse> {
    body.validate()?;

    let template = services::update(&state, id, body).await?;
    Ok(Json(ApiResponse::new(Some(template), "Template updated")))
}

pub async fn delete(State(state): State<AppState>, AppPath(id): AppPath<Uuid>) -> ApiResult<()> {
    services::delete(&state, id).await?;
    Ok(Json(ApiResponse::new(None, "Template deleted")))
}
