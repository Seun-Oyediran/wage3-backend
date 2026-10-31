use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use super::repo::TemplateRow;

// ---- Requests ----

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct CreateTemplateRequest {
    #[validate(length(min = 1, max = 100, message = "name must be 1-100 characters"))]
    pub name: String,

    #[validate(length(max = 500, message = "description must be at most 500 characters"))]
    pub description: Option<String>,
}

/// PATCH body: every field is optional, only the ones sent are changed.
#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct UpdateTemplateRequest {
    #[validate(length(min = 1, max = 100, message = "name must be 1-100 characters"))]
    pub name: Option<String>,

    #[validate(length(max = 500, message = "description must be at most 500 characters"))]
    pub description: Option<String>,
}

// ---- Responses ----

#[derive(Debug, Serialize)]
pub struct TemplateResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// The only way a DB row becomes API output, so new columns don't leak by accident.
impl From<TemplateRow> for TemplateResponse {
    fn from(row: TemplateRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            description: row.description,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}
