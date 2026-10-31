use chrono::{DateTime, Utc};
use sqlx::{PgExecutor, prelude::FromRow};
use uuid::Uuid;

#[derive(Debug, FromRow)]
pub struct PayoutRow {
    pub id: Uuid,
    #[allow(dead_code)]
    pub user_id: Uuid,
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
    pub prepared_at: Option<DateTime<Utc>>,
    pub paid_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
pub struct PayoutItemRow {
    pub id: Uuid,
    pub batch_id: Option<Uuid>,
    pub position: i32,
    pub name: Option<String>,
    pub email: Option<String>,
    pub wallet_address: String,
    pub amount: String,
}

#[derive(Debug)]
pub struct NewPayoutItem {
    pub name: Option<String>,
    pub email: Option<String>,
    pub wallet_address: String,
    pub amount: String,
}

#[derive(Debug, FromRow)]
pub struct PayoutBatchRow {
    pub id: Uuid,
    pub batch_index: i32,
    pub total_amount: String,
    pub recipient_count: i32,
    pub status: String,
    pub tx_hash: Option<String>,
    pub block_number: Option<i64>,
    pub submitted_at: Option<DateTime<Utc>>,
    pub confirmed_at: Option<DateTime<Utc>>,
}

/// A batch with its payout's status, for recording a sent transaction.
#[derive(Debug, FromRow)]
pub struct BatchForSubmit {
    pub status: String,
    pub tx_hash: Option<String>,
    pub payout_status: String,
}

pub struct NewPayout<'a> {
    pub user_id: Uuid,
    pub list_id: Option<Uuid>,
    pub name: &'a str,
    pub chain_id: i64,
    pub token_address: &'a str,
    pub token_symbol: &'a str,
    pub token_decimals: i32,
}

pub async fn insert_payout(
    db: impl PgExecutor<'_>,
    payout: NewPayout<'_>,
) -> Result<PayoutRow, sqlx::Error> {
    sqlx::query_as::<_, PayoutRow>(
        "INSERT INTO payouts (user_id, list_id, name, chain_id, token_address, token_symbol, token_decimals)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING id, user_id, list_id, name, chain_id, token_address, token_symbol, token_decimals,
                   sender_address, status, total_amount::TEXT AS total_amount, recipient_count,
                   prepared_at, paid_at, created_at, updated_at",
    )
    .bind(payout.user_id)
    .bind(payout.list_id)
    .bind(payout.name)
    .bind(payout.chain_id)
    .bind(payout.token_address)
    .bind(payout.token_symbol)
    .bind(payout.token_decimals)
    .fetch_one(db)
    .await
}

pub async fn insert_items(
    db: impl PgExecutor<'_>,
    payout_id: Uuid,
    items: &[NewPayoutItem],
) -> Result<(), sqlx::Error> {
    let positions: Vec<i32> = (0..items.len() as i32).collect();
    let names: Vec<Option<String>> = items.iter().map(|i| i.name.clone()).collect();
    let emails: Vec<Option<String>> = items.iter().map(|i| i.email.clone()).collect();
    let wallets: Vec<String> = items.iter().map(|i| i.wallet_address.clone()).collect();
    let amounts: Vec<String> = items.iter().map(|i| i.amount.clone()).collect();

    sqlx::query(
        "INSERT INTO payout_items (payout_id, position, name, email, wallet_address, amount)
         SELECT $1, t.position, t.name, t.email, t.wallet_address, t.amount::NUMERIC
         FROM UNNEST($2::INT[], $3::TEXT[], $4::TEXT[], $5::TEXT[], $6::TEXT[])
              AS t(position, name, email, wallet_address, amount)",
    )
    .bind(payout_id)
    .bind(positions)
    .bind(names)
    .bind(emails)
    .bind(wallets)
    .bind(amounts)
    .execute(db)
    .await?;

    Ok(())
}

pub async fn find_items_by_payout(
    db: impl PgExecutor<'_>,
    payout_id: Uuid,
) -> Result<Vec<PayoutItemRow>, sqlx::Error> {
    sqlx::query_as::<_, PayoutItemRow>(
        "SELECT id, batch_id, position, name, email, wallet_address, amount::TEXT AS amount
         FROM payout_items
         WHERE payout_id = $1
         ORDER BY position",
    )
    .bind(payout_id)
    .fetch_all(db)
    .await
}

pub async fn find_payout_by_id(
    db: impl PgExecutor<'_>,
    user_id: Uuid,
    id: Uuid,
) -> Result<Option<PayoutRow>, sqlx::Error> {
    sqlx::query_as::<_, PayoutRow>(
        "SELECT id, user_id, list_id, name, chain_id, token_address, token_symbol, token_decimals,
                sender_address, status, total_amount::TEXT AS total_amount, recipient_count,
                prepared_at, paid_at, created_at, updated_at
         FROM payouts
         WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(db)
    .await
}

pub async fn find_payout_for_update(
    db: impl PgExecutor<'_>,
    user_id: Uuid,
    id: Uuid,
) -> Result<Option<PayoutRow>, sqlx::Error> {
    sqlx::query_as::<_, PayoutRow>(
        "SELECT id, user_id, list_id, name, chain_id, token_address, token_symbol, token_decimals,
                sender_address, status, total_amount::TEXT AS total_amount, recipient_count,
                prepared_at, paid_at, created_at, updated_at
         FROM payouts
         WHERE id = $1 AND user_id = $2
         FOR UPDATE",
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(db)
    .await
}

pub async fn delete_items(db: impl PgExecutor<'_>, payout_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM payout_items WHERE payout_id = $1")
        .bind(payout_id)
        .execute(db)
        .await?;

    Ok(())
}

/// Bumps `updated_at` and returns the row as it is now.
pub async fn touch_payout(db: impl PgExecutor<'_>, id: Uuid) -> Result<PayoutRow, sqlx::Error> {
    sqlx::query_as::<_, PayoutRow>(
        "UPDATE payouts
         SET updated_at = NOW()
         WHERE id = $1
         RETURNING id, user_id, list_id, name, chain_id, token_address, token_symbol, token_decimals,
                   sender_address, status, total_amount::TEXT AS total_amount, recipient_count,
                   prepared_at, paid_at, created_at, updated_at",
    )
    .bind(id)
    .fetch_one(db)
    .await
}

pub async fn rename_payout(
    db: impl PgExecutor<'_>,
    user_id: Uuid,
    id: Uuid,
    name: &str,
) -> Result<Option<PayoutRow>, sqlx::Error> {
    sqlx::query_as::<_, PayoutRow>(
        "UPDATE payouts
         SET name = $3, updated_at = NOW()
         WHERE id = $1 AND user_id = $2
         RETURNING id, user_id, list_id, name, chain_id, token_address, token_symbol, token_decimals,
                   sender_address, status, total_amount::TEXT AS total_amount, recipient_count,
                   prepared_at, paid_at, created_at, updated_at",
    )
    .bind(id)
    .bind(user_id)
    .bind(name)
    .fetch_optional(db)
    .await
}

/// Only links a payout that isn't already tied to a list.
pub async fn link_list(
    db: impl PgExecutor<'_>,
    id: Uuid,
    list_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE payouts
         SET list_id = $2, updated_at = NOW()
         WHERE id = $1 AND list_id IS NULL",
    )
    .bind(id)
    .bind(list_id)
    .execute(db)
    .await?;

    Ok(())
}

/// One batch per `batch_size` people, in position order. Each gets a fresh UUID (its ref).
pub async fn create_batches(
    db: impl PgExecutor<'_>,
    payout_id: Uuid,
    batch_size: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO payout_batches (payout_id, batch_index, total_amount, recipient_count)
         SELECT $1, position / $2, SUM(amount), COUNT(*)
         FROM payout_items
         WHERE payout_id = $1
         GROUP BY position / $2",
    )
    .bind(payout_id)
    .bind(batch_size)
    .execute(db)
    .await?;

    Ok(())
}

pub async fn assign_items_to_batches(
    db: impl PgExecutor<'_>,
    payout_id: Uuid,
    batch_size: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE payout_items i
         SET batch_id = b.id
         FROM payout_batches b
         WHERE i.payout_id = $1
           AND b.payout_id = $1
           AND b.batch_index = i.position / $2",
    )
    .bind(payout_id)
    .bind(batch_size)
    .execute(db)
    .await?;

    Ok(())
}

pub async fn mark_payout_ready(
    db: impl PgExecutor<'_>,
    payout_id: Uuid,
    sender_address: &str,
) -> Result<PayoutRow, sqlx::Error> {
    sqlx::query_as::<_, PayoutRow>(
        "UPDATE payouts
         SET status = 'ready',
             sender_address = $2,
             total_amount = t.total,
             recipient_count = t.count,
             prepared_at = NOW(),
             updated_at = NOW()
         FROM (
             SELECT SUM(amount) AS total, COUNT(*)::INT AS count
             FROM payout_items
             WHERE payout_id = $1
         ) t
         WHERE payouts.id = $1
         RETURNING id, user_id, list_id, name, chain_id, token_address, token_symbol, token_decimals,
                   sender_address, status, total_amount::TEXT AS total_amount, recipient_count,
                   prepared_at, paid_at, created_at, updated_at",
    )
    .bind(payout_id)
    .bind(sender_address)
    .fetch_one(db)
    .await
}

pub async fn find_batches_by_payout(
    db: impl PgExecutor<'_>,
    payout_id: Uuid,
) -> Result<Vec<PayoutBatchRow>, sqlx::Error> {
    sqlx::query_as::<_, PayoutBatchRow>(
        "SELECT id, batch_index, total_amount::TEXT AS total_amount, recipient_count, status,
                tx_hash, block_number, submitted_at, confirmed_at
         FROM payout_batches
         WHERE payout_id = $1
         ORDER BY batch_index",
    )
    .bind(payout_id)
    .fetch_all(db)
    .await
}

/// Locks the batch only (not the payout), the same order the indexer uses.
pub async fn find_batch_for_update(
    db: impl PgExecutor<'_>,
    user_id: Uuid,
    payout_id: Uuid,
    batch_id: Uuid,
) -> Result<Option<BatchForSubmit>, sqlx::Error> {
    sqlx::query_as::<_, BatchForSubmit>(
        "SELECT pb.status, pb.tx_hash, p.status AS payout_status
         FROM payout_batches pb
         JOIN payouts p ON p.id = pb.payout_id
         WHERE pb.id = $1 AND pb.payout_id = $2 AND p.user_id = $3
         FOR UPDATE OF pb",
    )
    .bind(batch_id)
    .bind(payout_id)
    .bind(user_id)
    .fetch_optional(db)
    .await
}

/// A replaced hash (sped up in the wallet) keeps the first `submitted_at`.
pub async fn mark_batch_submitted(
    db: impl PgExecutor<'_>,
    batch_id: Uuid,
    tx_hash: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE payout_batches
         SET status = 'submitted',
             tx_hash = $2,
             submitted_at = COALESCE(submitted_at, NOW()),
             updated_at = NOW()
         WHERE id = $1",
    )
    .bind(batch_id)
    .bind(tx_hash)
    .execute(db)
    .await?;

    Ok(())
}

/// Only moves forward: a paid payout is never set back to submitted.
pub async fn mark_payout_submitted(
    db: impl PgExecutor<'_>,
    payout_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE payouts
         SET status = 'submitted', updated_at = NOW()
         WHERE id = $1 AND status = 'ready'",
    )
    .bind(payout_id)
    .execute(db)
    .await?;

    Ok(())
}
