use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use super::repo::{ListRow, ListSummaryRow, RecipientRow};

#[derive(Debug, Serialize)]
pub struct ListResponse {
    pub id: Uuid,
    pub name: String,
    pub recipient_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// The only way a DB row becomes API output, so new columns don't leak by accident.
impl From<ListSummaryRow> for ListResponse {
    fn from(row: ListSummaryRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            recipient_count: row.recipient_count,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct CreateListRequest {
    #[validate(length(
        min = 1,
        max = 100,
        message = "Name must be between 1 and 100 characters"
    ))]
    pub name: String,
    #[serde(default)]
    #[validate(length(max = 1000, message = "A list can have at most 1000 people"))]
    pub recipients: Vec<RecipientInput>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct RenameListRequest {
    #[validate(length(
        min = 1,
        max = 100,
        message = "Name must be between 1 and 100 characters"
    ))]
    pub name: String,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct AddRecipientsRequest {
    #[validate(length(min = 1, max = 1000, message = "Send between 1 and 1000 people"))]
    pub recipients: Vec<RecipientInput>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct ReplaceRecipientsRequest {
    #[validate(length(max = 1000, message = "A list can have at most 1000 people"))]
    pub recipients: Vec<RecipientInput>,
}

/// One person as the frontend sends it. Checked row by row in the service.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecipientInput {
    pub name: Option<String>,
    pub email: Option<String>,
    pub wallet_address: String,
    pub default_amount: Option<String>, // base units: "10000" = 0.01 USDC
}

#[derive(Debug, Serialize)]
pub struct RecipientResponse {
    pub id: Uuid,
    pub position: i32,
    pub name: Option<String>,
    pub email: Option<String>,
    pub wallet_address: String,
    pub default_amount: Option<String>,
}

impl From<RecipientRow> for RecipientResponse {
    fn from(row: RecipientRow) -> Self {
        Self {
            id: row.id,
            position: row.position,
            name: row.name,
            email: row.email,
            wallet_address: row.wallet_address,
            default_amount: row.default_amount,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ListDetailResponse {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub recipients: Vec<RecipientResponse>,
}

impl ListDetailResponse {
    pub fn new(list: ListRow, recipients: Vec<RecipientRow>) -> Self {
        Self {
            id: list.id,
            name: list.name,
            created_at: list.created_at,
            updated_at: list.updated_at,
            recipients: recipients.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct GetListRequestQuery {
    pub search: Option<String>,
}
