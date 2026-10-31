use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgExecutor};

use crate::handlers::users::repo::TokenTotalRow;

#[derive(Debug, FromRow)]
pub struct AppCountsRow {
    pub payouts_sent: i64,
    pub payments_made: i64,
    pub wallets_paid: i64,
    pub senders: i64,
}

#[derive(Debug, FromRow)]
pub struct HealthRow {
    pub users_total: i64,
    pub users_new: i64,
    pub active_senders: i64,
    pub batches_sending: i64,
    pub unmatched_events: i64,
    pub indexed_block: Option<i64>,
    pub indexer_updated_at: Option<DateTime<Utc>>,
}

/// Matched batches only: direct contract calls aren't wage3 volume.
pub async fn sum_sent_by_token(
    db: impl PgExecutor<'_>,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
) -> Result<Vec<TokenTotalRow>, sqlx::Error> {
    sqlx::query_as::<_, TokenTotalRow>(
        "SELECT p.token_address, p.token_symbol, p.token_decimals,
                SUM(b.total_amount)::TEXT AS amount
         FROM payout_batches b
         JOIN payouts p ON p.id = b.payout_id
         WHERE b.status = 'confirmed'
           AND ($1::TIMESTAMPTZ IS NULL OR COALESCE(b.submitted_at, b.confirmed_at) >= $1)
           AND ($2::TIMESTAMPTZ IS NULL OR COALESCE(b.submitted_at, b.confirmed_at) < $2)
         GROUP BY p.token_address, p.token_symbol, p.token_decimals
         ORDER BY p.token_address",
    )
    .bind(from)
    .bind(to)
    .fetch_all(db)
    .await
}

pub async fn count_sent(
    db: impl PgExecutor<'_>,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
) -> Result<AppCountsRow, sqlx::Error> {
    sqlx::query_as::<_, AppCountsRow>(
        "SELECT COUNT(DISTINCT p.id) AS payouts_sent,
                COUNT(i.id) AS payments_made,
                COUNT(DISTINCT i.wallet_address) AS wallets_paid,
                COUNT(DISTINCT p.user_id) AS senders
         FROM payout_batches b
         JOIN payouts p ON p.id = b.payout_id
         JOIN payout_items i ON i.batch_id = b.id
         WHERE b.status = 'confirmed'
           AND ($1::TIMESTAMPTZ IS NULL OR COALESCE(b.submitted_at, b.confirmed_at) >= $1)
           AND ($2::TIMESTAMPTZ IS NULL OR COALESCE(b.submitted_at, b.confirmed_at) < $2)",
    )
    .bind(from)
    .bind(to)
    .fetch_one(db)
    .await
}

pub async fn count_payouts_by_status(
    db: impl PgExecutor<'_>,
) -> Result<Vec<(String, i64)>, sqlx::Error> {
    sqlx::query_as("SELECT status, COUNT(*) FROM payouts GROUP BY status")
        .fetch_all(db)
        .await
}

/// An event is unmatched when no confirmed batch has its ref and tx hash,
/// e.g. a direct contract call or someone reusing a public ref.
pub async fn find_health(
    db: impl PgExecutor<'_>,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
    cursor_name: &str,
) -> Result<HealthRow, sqlx::Error> {
    sqlx::query_as::<_, HealthRow>(
        "SELECT
             (SELECT COUNT(*) FROM users) AS users_total,
             (SELECT COUNT(*) FROM users
              WHERE ($1::TIMESTAMPTZ IS NULL OR created_at >= $1)
                AND ($2::TIMESTAMPTZ IS NULL OR created_at < $2)) AS users_new,
             (SELECT COUNT(DISTINCT p.user_id)
              FROM payout_batches b
              JOIN payouts p ON p.id = b.payout_id
              WHERE b.status = 'confirmed'
                AND COALESCE(b.submitted_at, b.confirmed_at) >= NOW() - INTERVAL '30 days') AS active_senders,
             (SELECT COUNT(*) FROM payout_batches WHERE status = 'submitted') AS batches_sending,
             (SELECT COUNT(*) FROM onchain_batches ob
              WHERE NOT EXISTS (
                  SELECT 1 FROM payout_batches pb
                  WHERE pb.id = ob.batch_ref AND pb.status = 'confirmed' AND pb.tx_hash = ob.tx_hash
              )) AS unmatched_events,
             (SELECT last_block FROM indexer_cursors WHERE name = $3) AS indexed_block,
             (SELECT updated_at FROM indexer_cursors WHERE name = $3) AS indexer_updated_at",
    )
    .bind(from)
    .bind(to)
    .bind(cursor_name)
    .fetch_one(db)
    .await
}
