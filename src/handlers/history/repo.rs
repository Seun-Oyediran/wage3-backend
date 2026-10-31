use chrono::{DateTime, Utc};
use sqlx::{PgExecutor, prelude::FromRow};
use uuid::Uuid;

#[derive(Debug, FromRow)]
pub struct SentPayoutRow {
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

pub struct SentFilters<'a> {
    pub user_id: Uuid,
    pub status: Option<&'a str>,
    pub list_id: Option<Uuid>,
    pub search: Option<&'a str>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

/// Shared by the page and the count, so both always apply the same filters.
/// `sent_at` is when the money first left: the hint's time, or the confirmation if there was no hint.
macro_rules! sent_payouts {
    () => {
        "WITH sent AS (
             SELECT p.id, p.list_id, p.name, p.chain_id, p.token_address, p.token_symbol,
                    p.token_decimals, p.sender_address, p.status,
                    p.total_amount::TEXT AS total_amount, p.recipient_count,
                    COUNT(b.id)::INT AS batch_count,
                    (COUNT(b.id) FILTER (WHERE b.status = 'confirmed'))::INT AS confirmed_batch_count,
                    MIN(COALESCE(b.submitted_at, b.confirmed_at))
                        FILTER (WHERE b.status IN ('submitted', 'confirmed')) AS sent_at,
                    p.paid_at, p.created_at
             FROM payouts p
             JOIN payout_batches b ON b.payout_id = p.id
             WHERE p.user_id = $1
               AND p.status IN ('submitted', 'paid', 'partially_paid')
               AND ($2::TEXT IS NULL OR p.status = $2)
               AND ($3::UUID IS NULL OR p.list_id = $3)
               AND ($4::TEXT IS NULL OR p.name ILIKE '%' || $4 || '%')
             GROUP BY p.id
         ),
         filtered AS (
             SELECT * FROM sent
             WHERE ($5::TIMESTAMPTZ IS NULL OR sent_at >= $5)
               AND ($6::TIMESTAMPTZ IS NULL OR sent_at < $6)
         )"
    };
}

pub async fn find_sent(
    db: impl PgExecutor<'_>,
    f: &SentFilters<'_>,
    limit: i64,
    offset: i64,
) -> Result<Vec<SentPayoutRow>, sqlx::Error> {
    sqlx::query_as::<_, SentPayoutRow>(concat!(
        sent_payouts!(),
        " SELECT * FROM filtered
          ORDER BY sent_at DESC NULLS LAST, id DESC
          LIMIT $7 OFFSET $8"
    ))
    .bind(f.user_id)
    .bind(f.status)
    .bind(f.list_id)
    .bind(f.search)
    .bind(f.from)
    .bind(f.to)
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await
}

pub async fn count_sent(db: impl PgExecutor<'_>, f: &SentFilters<'_>) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(concat!(sent_payouts!(), " SELECT COUNT(*) FROM filtered"))
        .bind(f.user_id)
        .bind(f.status)
        .bind(f.list_id)
        .bind(f.search)
        .bind(f.from)
        .bind(f.to)
        .fetch_one(db)
        .await
}
