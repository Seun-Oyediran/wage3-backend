use sqlx::PgExecutor;
use uuid::Uuid;

#[derive(Debug)]
pub struct LogMeta {
    pub chain_id: i64,
    pub tx_hash: String,
    pub log_index: i32,
    pub block_number: i64,
    pub block_time: i64, // Unix seconds
}

#[derive(Debug)]
pub struct OnchainBatch {
    pub meta: LogMeta,
    pub batch_ref: Uuid,
    pub sender: String,
    pub token: String,
    pub total_amount: String, // uint256 as a decimal string
    pub recipient_count: i32,
}

#[derive(Debug)]
pub struct OnchainPayment {
    pub meta: LogMeta,
    pub batch_ref: Uuid,
    pub sender: String,
    pub recipient: String,
    pub token: String,
    pub amount: String,
}

pub async fn get_or_init_cursor(
    db: impl PgExecutor<'_>,
    name: &str,
    initial: i64,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "INSERT INTO indexer_cursors (name, last_block)
         VALUES ($1, $2)
         ON CONFLICT (name) DO UPDATE SET name = EXCLUDED.name
         RETURNING last_block",
    )
    .bind(name)
    .bind(initial)
    .fetch_one(db)
    .await
}

pub async fn set_cursor(
    db: impl PgExecutor<'_>,
    name: &str,
    last_block: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE indexer_cursors SET last_block = $2, updated_at = NOW() WHERE name = $1")
        .bind(name)
        .bind(last_block)
        .execute(db)
        .await?;

    Ok(())
}

pub async fn insert_batch(db: impl PgExecutor<'_>, b: &OnchainBatch) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO onchain_batches
             (chain_id, batch_ref, sender_address, token_address, total_amount, recipient_count,
              tx_hash, log_index, block_number, block_time)
         VALUES ($1, $2, $3, $4, $5::NUMERIC, $6, $7, $8, $9, to_timestamp($10))
         ON CONFLICT (chain_id, tx_hash, log_index) DO NOTHING",
    )
    .bind(b.meta.chain_id)
    .bind(b.batch_ref)
    .bind(&b.sender)
    .bind(&b.token)
    .bind(&b.total_amount)
    .bind(b.recipient_count)
    .bind(&b.meta.tx_hash)
    .bind(b.meta.log_index)
    .bind(b.meta.block_number)
    .bind(b.meta.block_time)
    .execute(db)
    .await?;

    Ok(())
}

pub async fn insert_payment(
    db: impl PgExecutor<'_>,
    p: &OnchainPayment,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO onchain_payments
             (chain_id, batch_ref, sender_address, recipient_address, token_address, amount,
              tx_hash, log_index, block_number, block_time)
         VALUES ($1, $2, $3, $4, $5, $6::NUMERIC, $7, $8, $9, to_timestamp($10))
         ON CONFLICT (chain_id, tx_hash, log_index) DO NOTHING",
    )
    .bind(p.meta.chain_id)
    .bind(p.batch_ref)
    .bind(&p.sender)
    .bind(&p.recipient)
    .bind(&p.token)
    .bind(&p.amount)
    .bind(&p.meta.tx_hash)
    .bind(p.meta.log_index)
    .bind(p.meta.block_number)
    .bind(p.meta.block_time)
    .execute(db)
    .await?;

    Ok(())
}
