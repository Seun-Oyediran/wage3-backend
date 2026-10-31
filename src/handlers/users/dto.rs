use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::repo::{SentCountsRow, TokenTotalRow, UserRow};

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub full_name: Option<String>,
    pub email: String,
    pub avatar_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// The only way a DB row becomes API output, so new columns don't leak by accident.
impl From<UserRow> for UserResponse {
    fn from(row: UserRow) -> Self {
        Self {
            id: row.id,
            full_name: row.full_name,
            avatar_url: row.avatar_url,
            email: row.email,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct StatsQuery {
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
}

#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub total_sent: Vec<TokenTotal>,
    pub payouts_sent: i64,
    pub wallets_paid: i64,
    pub last_sent_at: Option<DateTime<Utc>>,
}

/// Per token: amounts in different tokens can't be added together.
#[derive(Debug, Clone, Serialize)]
pub struct TokenTotal {
    pub token_address: String,
    pub token_symbol: Option<String>,
    pub token_decimals: Option<i32>,
    pub amount: String, // base units
}

impl StatsResponse {
    pub fn new(totals: Vec<TokenTotalRow>, counts: SentCountsRow) -> Self {
        Self {
            total_sent: totals.into_iter().map(Into::into).collect(),
            payouts_sent: counts.payouts_sent,
            wallets_paid: counts.wallets_paid,
            last_sent_at: counts.last_sent_at,
        }
    }
}

impl From<TokenTotalRow> for TokenTotal {
    fn from(t: TokenTotalRow) -> Self {
        Self {
            token_address: t.token_address,
            token_symbol: t.token_symbol,
            token_decimals: t.token_decimals,
            amount: t.amount,
        }
    }
}
