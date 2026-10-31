use sqlx::{FromRow, PgConnection, PgPool};
use uuid::Uuid;

use super::repo::OnchainBatch;

/// The parts of a BatchPayment event needed to match it to a payout batch.
#[derive(Debug, FromRow)]
pub struct BatchEvent {
    pub batch_ref: Uuid,
    pub chain_id: i64,
    pub tx_hash: String,
    pub block_number: i64,
    pub block_time: i64, // Unix seconds
    pub sender_address: String,
    pub token_address: String,
}

impl From<&OnchainBatch> for BatchEvent {
    fn from(b: &OnchainBatch) -> Self {
        Self {
            batch_ref: b.batch_ref,
            chain_id: b.meta.chain_id,
            tx_hash: b.meta.tx_hash.clone(),
            block_number: b.meta.block_number,
            block_time: b.meta.block_time,
            sender_address: b.sender.clone(),
            token_address: b.token.clone(),
        }
    }
}

/// What the app expected for this batch.
#[derive(Debug, FromRow)]
struct ExpectedBatch {
    payout_id: Uuid,
    status: String,
    chain_id: i64,
    sender_address: Option<String>,
    token_address: String,
}

/// One recipient and amount, from either side. Compared in order.
#[derive(Debug, FromRow, PartialEq)]
struct Transfer {
    address: String,
    amount: String,
}

/// Marks the payout batch with this ref as confirmed if the event matches it exactly,
/// and the payout as paid once all its batches are confirmed.
/// Events that aren't ours, or don't match, are left alone.
pub async fn reconcile(conn: &mut PgConnection, ev: &BatchEvent) -> Result<(), sqlx::Error> {
    let expected = sqlx::query_as::<_, ExpectedBatch>(
        "SELECT pb.payout_id, pb.status, p.chain_id, p.sender_address, p.token_address
         FROM payout_batches pb
         JOIN payouts p ON p.id = pb.payout_id
         WHERE pb.id = $1
         FOR UPDATE OF pb",
    )
    .bind(ev.batch_ref)
    .fetch_optional(&mut *conn)
    .await?;

    // Not one of our refs: someone used the contract directly, or a test script.
    let Some(expected) = expected else {
        return Ok(());
    };

    // Already matched (e.g. the range was indexed again).
    if expected.status == "confirmed" {
        return Ok(());
    }

    // The ref is public on-chain, so anyone can reuse it from their own wallet.
    if expected.chain_id != ev.chain_id
        || expected.sender_address.as_deref() != Some(ev.sender_address.as_str())
        || expected.token_address != ev.token_address
    {
        tracing::warn!(
            batch_ref = %ev.batch_ref,
            tx_hash = %ev.tx_hash,
            "batch event doesn't match the payout's chain, sender or token"
        );
        return Ok(());
    }

    let wanted = sqlx::query_as::<_, Transfer>(
        "SELECT wallet_address AS address, amount::TEXT AS amount
         FROM payout_items
         WHERE batch_id = $1
         ORDER BY position",
    )
    .bind(ev.batch_ref)
    .fetch_all(&mut *conn)
    .await?;

    // Logs come out in the same order as the recipients array that was sent.
    let paid = sqlx::query_as::<_, Transfer>(
        "SELECT recipient_address AS address, amount::TEXT AS amount
         FROM onchain_payments
         WHERE chain_id = $1 AND tx_hash = $2 AND batch_ref = $3 AND sender_address = $4
         ORDER BY log_index",
    )
    .bind(ev.chain_id)
    .bind(&ev.tx_hash)
    .bind(ev.batch_ref)
    .bind(&ev.sender_address)
    .fetch_all(&mut *conn)
    .await?;

    if wanted != paid {
        tracing::warn!(
            batch_ref = %ev.batch_ref,
            tx_hash = %ev.tx_hash,
            expected = wanted.len(),
            got = paid.len(),
            "batch event recipients or amounts don't match the payout"
        );
        return Ok(());
    }

    sqlx::query(
        "UPDATE payout_batches
         SET status = 'confirmed',
             tx_hash = $2,
             block_number = $3,
             confirmed_at = to_timestamp($4),
             updated_at = NOW()
         WHERE id = $1",
    )
    .bind(ev.batch_ref)
    .bind(&ev.tx_hash)
    .bind(ev.block_number)
    .bind(ev.block_time)
    .execute(&mut *conn)
    .await?;

    // All batches confirmed → paid. Otherwise still in progress.
    // The chain is the truth: this applies even if the payout was cancelled in the app.
    sqlx::query(
        "WITH progress AS (
             SELECT BOOL_AND(status = 'confirmed') AS all_confirmed,
                    MAX(confirmed_at) AS last_confirmed_at
             FROM payout_batches
             WHERE payout_id = $1
         )
         UPDATE payouts
         SET status = CASE WHEN progress.all_confirmed THEN 'paid' ELSE 'submitted' END,
             paid_at = CASE WHEN progress.all_confirmed THEN progress.last_confirmed_at END,
             updated_at = NOW()
         FROM progress
         WHERE payouts.id = $1",
    )
    .bind(expected.payout_id)
    .execute(&mut *conn)
    .await?;

    tracing::info!(batch_ref = %ev.batch_ref, tx_hash = %ev.tx_hash, "payout batch confirmed");

    Ok(())
}

/// Matches events that were indexed before their batch was matched,
/// e.g. events saved before this code existed. Run once at startup.
pub async fn reconcile_unmatched(db: &PgPool) -> Result<(), sqlx::Error> {
    let events = sqlx::query_as::<_, BatchEvent>(
        "SELECT ob.batch_ref, ob.chain_id, ob.tx_hash, ob.block_number,
                EXTRACT(EPOCH FROM ob.block_time)::BIGINT AS block_time,
                ob.sender_address, ob.token_address
         FROM onchain_batches ob
         JOIN payout_batches pb ON pb.id = ob.batch_ref
         JOIN payouts p ON p.id = pb.payout_id
         WHERE pb.status <> 'confirmed'
           -- Spoofed refs were already warned about when first indexed.
           AND ob.chain_id = p.chain_id
           AND ob.sender_address = p.sender_address
           AND ob.token_address = p.token_address
         ORDER BY ob.block_number, ob.log_index",
    )
    .fetch_all(db)
    .await?;

    for ev in &events {
        let mut tx = db.begin().await?;
        reconcile(&mut tx, ev).await?;
        tx.commit().await?;
    }

    Ok(())
}
