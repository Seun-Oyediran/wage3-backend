use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::repo::SentPayoutRow;

/// No `deny_unknown_fields`: `page` and `limit` share the query string.
#[derive(Debug, Deserialize)]
pub struct SentHistoryQuery {
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    pub status: Option<String>,
    pub list_id: Option<Uuid>,
    pub search: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SentPayoutResponse {
    pub id: Uuid,
    pub list_id: Option<Uuid>,
    pub name: String,
    pub chain_id: i64,
    pub token_address: String,
    pub token_symbol: Option<String>,
    pub token_decimals: Option<i32>,
    pub sender_address: Option<String>,
    pub status: String,
    pub total_amount: Option<String>,
    pub recipient_count: Option<i32>,
    pub batch_count: i32,
    pub confirmed_batch_count: i32,
    pub sent_at: Option<DateTime<Utc>>,
    pub paid_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<SentPayoutRow> for SentPayoutResponse {
    fn from(row: SentPayoutRow) -> Self {
        Self {
            id: row.id,
            list_id: row.list_id,
            name: row.name,
            chain_id: row.chain_id,
            token_address: row.token_address,
            token_symbol: row.token_symbol,
            token_decimals: row.token_decimals,
            sender_address: row.sender_address,
            status: row.status,
            total_amount: row.total_amount,
            recipient_count: row.recipient_count,
            batch_count: row.batch_count,
            confirmed_batch_count: row.confirmed_batch_count,
            sent_at: row.sent_at,
            paid_at: row.paid_at,
            created_at: row.created_at,
        }
    }
}
