use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgExecutor};
use uuid::Uuid;

#[derive(Debug, FromRow)]
pub struct UserRow {
    pub id: Uuid,
    pub full_name: Option<String>,
    pub email: String,
    #[allow(dead_code)]
    google_sub: Option<String>,
    pub avatar_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub async fn find_by_id(db: impl PgExecutor<'_>, id: Uuid) -> Result<Option<UserRow>, sqlx::Error> {
    sqlx::query_as::<_, UserRow>(
        "SELECT id, full_name, email, google_sub, avatar_url, created_at, updated_at
         FROM users
         WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(db)
    .await
}

#[derive(Debug, FromRow)]
pub struct TokenTotalRow {
    pub token_address: String,
    pub token_symbol: Option<String>,
    pub token_decimals: Option<i32>,
    pub amount: String,
}

#[derive(Debug, FromRow)]
pub struct SentCountsRow {
    pub payouts_sent: i64,
    pub wallets_paid: i64,
    pub last_sent_at: Option<DateTime<Utc>>,
}

/// Confirmed batches only. A batch's time is when it was sent, or confirmed if there was no hint.
pub async fn sum_sent_by_token(
    db: impl PgExecutor<'_>,
    user_id: Uuid,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
) -> Result<Vec<TokenTotalRow>, sqlx::Error> {
    sqlx::query_as::<_, TokenTotalRow>(
        "SELECT p.token_address, p.token_symbol, p.token_decimals,
                SUM(b.total_amount)::TEXT AS amount
         FROM payouts p
         JOIN payout_batches b ON b.payout_id = p.id
         WHERE p.user_id = $1
           AND b.status = 'confirmed'
           AND ($2::TIMESTAMPTZ IS NULL OR COALESCE(b.submitted_at, b.confirmed_at) >= $2)
           AND ($3::TIMESTAMPTZ IS NULL OR COALESCE(b.submitted_at, b.confirmed_at) < $3)
         GROUP BY p.token_address, p.token_symbol, p.token_decimals
         ORDER BY p.token_address",
    )
    .bind(user_id)
    .bind(from)
    .bind(to)
    .fetch_all(db)
    .await
}

/// Counted across all tokens: one wallet paid in two tokens is still one wallet.
pub async fn count_sent(
    db: impl PgExecutor<'_>,
    user_id: Uuid,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
) -> Result<SentCountsRow, sqlx::Error> {
    sqlx::query_as::<_, SentCountsRow>(
        "SELECT COUNT(DISTINCT p.id) AS payouts_sent,
                COUNT(DISTINCT i.wallet_address) AS wallets_paid,
                MAX(COALESCE(b.submitted_at, b.confirmed_at)) AS last_sent_at
         FROM payouts p
         JOIN payout_batches b ON b.payout_id = p.id
         JOIN payout_items i ON i.batch_id = b.id
         WHERE p.user_id = $1
           AND b.status = 'confirmed'
           AND ($2::TIMESTAMPTZ IS NULL OR COALESCE(b.submitted_at, b.confirmed_at) >= $2)
           AND ($3::TIMESTAMPTZ IS NULL OR COALESCE(b.submitted_at, b.confirmed_at) < $3)",
    )
    .bind(user_id)
    .bind(from)
    .bind(to)
    .fetch_one(db)
    .await
}
