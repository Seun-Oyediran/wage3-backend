use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::repo::{AppCountsRow, HealthRow};
use crate::handlers::users::{dto::TokenTotal, repo::TokenTotalRow};

#[derive(Debug, Deserialize)]
pub struct AdminStatsQuery {
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
}

/// Confirmed batches only, across all users. Nothing here points to a single user.
#[derive(Debug, Clone, Serialize)]
pub struct AppStatsResponse {
    pub total_sent: Vec<TokenTotal>,
    pub payouts_sent: i64,
    pub payments_made: i64,
    pub wallets_paid: i64,
    pub senders: i64,
}

impl AppStatsResponse {
    pub fn new(totals: Vec<TokenTotalRow>, counts: AppCountsRow) -> Self {
        Self {
            total_sent: totals.into_iter().map(Into::into).collect(),
            payouts_sent: counts.payouts_sent,
            payments_made: counts.payments_made,
            wallets_paid: counts.wallets_paid,
            senders: counts.senders,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AdminStatsResponse {
    pub volume: AppStatsResponse,
    pub users: UserStats,
    pub payouts_by_status: PayoutStatusCounts,
    pub health: Health,
}

#[derive(Debug, Serialize)]
pub struct UserStats {
    pub total: i64,
    pub new: i64,            // signed up in the date range
    pub active_senders: i64, // sent money in the last 30 days
}

#[derive(Debug, Default, Serialize)]
pub struct PayoutStatusCounts {
    pub draft: i64,
    pub ready: i64,
    pub submitted: i64,
    pub paid: i64,
    pub partially_paid: i64,
    pub cancelled: i64,
}

impl PayoutStatusCounts {
    pub fn new(rows: Vec<(String, i64)>) -> Self {
        let mut counts = Self::default();
        for (status, n) in rows {
            match status.as_str() {
                "draft" => counts.draft = n,
                "ready" => counts.ready = n,
                "submitted" => counts.submitted = n,
                "paid" => counts.paid = n,
                "partially_paid" => counts.partially_paid = n,
                "cancelled" => counts.cancelled = n,
                _ => {}
            }
        }
        counts
    }
}

#[derive(Debug, Serialize)]
pub struct Health {
    pub payments_enabled: bool,
    pub batches_sending: i64,
    pub unmatched_events: i64, // contract calls that matched no payout batch
    pub indexed_block: Option<i64>,
    pub indexer_updated_at: Option<DateTime<Utc>>, // stops moving if the indexer is stuck
}

impl Health {
    pub fn new(row: &HealthRow, payments_enabled: bool) -> Self {
        Self {
            payments_enabled,
            batches_sending: row.batches_sending,
            unmatched_events: row.unmatched_events,
            indexed_block: row.indexed_block,
            indexer_updated_at: row.indexer_updated_at,
        }
    }
}
