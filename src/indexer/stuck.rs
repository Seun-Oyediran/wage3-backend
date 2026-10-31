use std::time::Duration;

use alloy::{
    primitives::B256,
    providers::{DynProvider, Provider},
};
use chrono::{DateTime, TimeDelta, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use super::BoxError;

const CHECK_EVERY: Duration = Duration::from_secs(60);

/// A hash the RPC still doesn't know this long after it was saved was never sent, or was replaced.
/// Timed from `updated_at`, so a sped-up hash gets the full wait.
const NOT_FOUND_AFTER: TimeDelta = TimeDelta::minutes(30);

#[derive(Debug, FromRow)]
struct SubmittedBatch {
    id: Uuid,
    tx_hash: String,
    hash_saved_at: DateTime<Utc>,
}

/// Puts batches whose hash will never confirm back to `ready`, so they can be sent again.
/// Resending is safe: the contract accepts a ref only once per wallet.
pub fn spawn_stuck_batch_job(
    db: PgPool,
    provider: DynProvider,
    chain_id: i64,
    cursor_name: String,
) {
    tokio::spawn(async move {
        loop {
            if let Err(e) = check_submitted(&db, &provider, chain_id, &cursor_name).await {
                tracing::error!("stuck batch check failed: {}", e);
            }
            tokio::time::sleep(CHECK_EVERY).await;
        }
    });
}

async fn check_submitted(
    db: &PgPool,
    provider: &DynProvider,
    chain_id: i64,
    cursor_name: &str,
) -> Result<(), BoxError> {
    let batches = sqlx::query_as::<_, SubmittedBatch>(
        "SELECT pb.id, pb.tx_hash, pb.updated_at AS hash_saved_at
         FROM payout_batches pb
         JOIN payouts p ON p.id = pb.payout_id
         WHERE pb.status = 'submitted'
           AND pb.tx_hash IS NOT NULL
           AND p.chain_id = $1",
    )
    .bind(chain_id)
    .fetch_all(db)
    .await?;

    if batches.is_empty() {
        return Ok(());
    }

    let indexed_to: Option<i64> =
        sqlx::query_scalar("SELECT last_block FROM indexer_cursors WHERE name = $1")
            .bind(cursor_name)
            .fetch_optional(db)
            .await?;

    for b in batches {
        let Ok(hash) = b.tx_hash.parse::<B256>() else {
            continue;
        };

        let reason = match provider.get_transaction_receipt(hash).await {
            Ok(Some(r)) if !r.status() => Some("transaction reverted"),
            // The indexer has passed its block and didn't match it: it's not this batch's.
            Ok(Some(r)) => r
                .block_number
                .filter(|n| indexed_to.is_some_and(|to| to >= *n as i64))
                .map(|_| "transaction doesn't match the batch"),
            Ok(None) => match provider.get_transaction_by_hash(hash).await {
                Ok(Some(_)) => None, // pending: speeding it up in the wallet is up to the user
                Ok(None) if Utc::now() - b.hash_saved_at > NOT_FOUND_AFTER => {
                    Some("transaction not found")
                }
                Ok(None) => None,
                Err(e) => {
                    tracing::warn!(tx_hash = %b.tx_hash, "stuck check: tx lookup failed: {}", e);
                    None
                }
            },
            Err(e) => {
                tracing::warn!(tx_hash = %b.tx_hash, "stuck check: receipt lookup failed: {}", e);
                None
            }
        };

        if let Some(reason) = reason {
            reset_batch(db, b.id, &b.tx_hash, reason).await?;
        }
    }

    Ok(())
}

/// Locks the batch, then the payout: the same order as the indexer and the hint endpoint.
async fn reset_batch(
    db: &PgPool,
    id: Uuid,
    tx_hash: &str,
    reason: &str,
) -> Result<(), sqlx::Error> {
    let mut tx = db.begin().await?;

    // Confirmed, or given a new hash, since it was loaded: leave it.
    let payout_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT payout_id FROM payout_batches
         WHERE id = $1 AND status = 'submitted' AND tx_hash = $2
         FOR UPDATE",
    )
    .bind(id)
    .bind(tx_hash)
    .fetch_optional(&mut *tx)
    .await?;

    let Some(payout_id) = payout_id else {
        return Ok(());
    };

    sqlx::query(
        "UPDATE payout_batches
         SET status = 'ready', tx_hash = NULL, submitted_at = NULL, updated_at = NOW()
         WHERE id = $1",
    )
    .bind(id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "UPDATE payouts
         SET status = 'ready', updated_at = NOW()
         WHERE id = $1
           AND status = 'submitted'
           AND NOT EXISTS (
               SELECT 1 FROM payout_batches
               WHERE payout_id = $1 AND status IN ('submitted', 'confirmed')
           )",
    )
    .bind(payout_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    tracing::info!(batch_ref = %id, tx_hash, reason, "stuck batch reset to ready");

    Ok(())
}
