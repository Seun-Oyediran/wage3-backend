//! Business rules for templates. Takes plain values, returns DTOs or AppError.
//! No axum types here, so this layer can be unit-tested without HTTP.

use uuid::Uuid;

use super::{
    dto::{CreateTemplateRequest, TemplateResponse, UpdateTemplateRequest},
    repo,
};
use crate::{
    error::AppError,
    response::{PaginatedList, PaginatedRequest, PaginatedResponse},
    state::AppState,
};

pub async fn create(
    state: &AppState,
    req: CreateTemplateRequest,
) -> Result<TemplateResponse, AppError> {
    let name = req.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("name cannot be blank".to_string()));
    }

    let description = normalize_optional(req.description.as_deref());

    let row = repo::insert(&state.db, name, description).await?;
    Ok(row.into())
}

pub async fn get(state: &AppState, id: Uuid) -> Result<TemplateResponse, AppError> {
    repo::find_by_id(&state.db, id)
        .await?
        .map(Into::into)
        .ok_or_else(not_found)
}

pub async fn list(
    state: &AppState,
    query: &PaginatedRequest,
) -> Result<PaginatedList<TemplateResponse>, AppError> {
    let (page, limit) = (query.page(), query.limit());

    // Both reads in one transaction so `total` matches the page.
    let mut tx = state.db.begin().await?;
    let rows = repo::list(&mut *tx, limit, query.offset()).await?;
    let total = repo::count(&mut *tx).await?;
    tx.commit().await?;

    Ok(PaginatedList {
        items: rows.into_iter().map(Into::into).collect(),
        pagination: PaginatedResponse::new(total, limit, page),
    })
}

pub async fn update(
    state: &AppState,
    id: Uuid,
    req: UpdateTemplateRequest,
) -> Result<TemplateResponse, AppError> {
    let name = req.name.as_deref().map(str::trim);
    if name == Some("") {
        return Err(AppError::BadRequest("name cannot be blank".to_string()));
    }

    let description = normalize_optional(req.description.as_deref());

    repo::update(&state.db, id, name, description)
        .await?
        .map(Into::into)
        .ok_or_else(not_found)
}

pub async fn delete(state: &AppState, id: Uuid) -> Result<(), AppError> {
    if repo::delete(&state.db, id).await? {
        Ok(())
    } else {
        Err(not_found())
    }
}

fn not_found() -> AppError {
    AppError::NotFound("Template not found".to_string())
}

/// Trims the value and treats an empty string as "not provided".
fn normalize_optional(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|v| !v.is_empty())
}
