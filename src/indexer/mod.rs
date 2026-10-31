use std::{collections::HashMap, time::Duration};

use alloy::{
    eips::BlockNumberOrTag,
    primitives::Address,
    providers::{DynProvider, Provider, ProviderBuilder},
    rpc::types::{Filter, Log},
    sol,
    sol_types::SolEvent,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::indexer::{
    matching::BatchEvent,
    repo::{LogMeta, OnchainBatch, OnchainPayment},
};

mod matching;
mod repo;
mod stuck;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

sol! {
    event BatchPayment(bytes16 indexed batchRef, address indexed sender, address indexed token, uint256 totalAmount, uint256 recipientCount);
    event PaymentSent(bytes16 indexed batchRef, address indexed sender, address indexed recipient, address token, uint256 amount);
}

pub struct IndexerConfig {
    pub rpc_url: String,
    pub chain_id: i64,
    pub contract: Address,
    pub start_block: u64,
    pub max_range: u64,
}

enum ChainEvent {
    Batch(OnchainBatch),
    Payment(OnchainPayment),
}

pub fn spawn_indexer(db: PgPool, config: IndexerConfig) -> Result<(), BoxError> {
    let provider = ProviderBuilder::new()
        .connect_http(config.rpc_url.parse()?)
        .erased();
    let cursor_name = format!("batcher:{}:{:#x}", config.chain_id, config.contract);

    stuck::spawn_stuck_batch_job(
        db.clone(),
        provider.clone(),
        config.chain_id,
        cursor_name.clone(),
    );

    tokio::spawn(async move {
        // Events saved before they could be matched (e.g. before matching existed).
        if let Err(e) = matching::reconcile_unmatched(&db).await {
            tracing::error!("matching old events failed: {}", e);
        }

        loop {
            match index_next_range(&db, &provider, &config, &cursor_name).await {
                // Caught up: wait for new blocks (Base makes one every 2s).
                Ok(true) => tokio::time::sleep(Duration::from_secs(5)).await,
                // Still backfilling: keep going, gently, for the public RPC's rate limit.
                Ok(false) => tokio::time::sleep(Duration::from_millis(250)).await,
                Err(e) => {
                    tracing::error!("indexer failed, retrying: {}", e);
                    tokio::time::sleep(Duration::from_secs(10)).await;
                }
            }
        }
    });

    Ok(())
}

async fn index_next_range(
    db: &PgPool,
    provider: &DynProvider,
    config: &IndexerConfig,
    cursor_name: &str,
) -> Result<bool, BoxError> {
    let last_block =
        repo::get_or_init_cursor(db, cursor_name, config.start_block as i64 - 1).await? as u64;

    // "Safe" blocks are already posted to Ethereum, so they won't be reorged away.
    let safe_head = provider
        .get_block_by_number(BlockNumberOrTag::Safe)
        .await?
        .ok_or("safe block not available")?
        .header
        .number;

    if last_block >= safe_head {
        return Ok(true);
    }

    let from = last_block + 1;
    let to = (from + config.max_range - 1).min(safe_head);

    let filter = Filter::new()
        .address(config.contract)
        .event_signature(vec![
            BatchPayment::SIGNATURE_HASH,
            PaymentSent::SIGNATURE_HASH,
        ])
        .from_block(from)
        .to_block(to);

    let logs = provider.get_logs(&filter).await?;

    // Decode (and fetch block times) before opening the DB transaction,
    // so it isn't held open while we wait on the RPC.
    let mut block_times = HashMap::new();
    let mut events = Vec::with_capacity(logs.len());
    for log in &logs {
        if let Some(event) = decode_log(provider, &mut block_times, config.chain_id, log).await? {
            events.push(event);
        }
    }

    // Events and cursor move together: either the whole range is saved or none of it.
    let mut tx = db.begin().await?;
    for event in &events {
        match event {
            ChainEvent::Batch(b) => repo::insert_batch(&mut *tx, b).await?,
            ChainEvent::Payment(p) => repo::insert_payment(&mut *tx, p).await?,
        }
    }

    // After all inserts: a batch's PaymentSent rows must be saved before it's matched.
    for event in &events {
        if let ChainEvent::Batch(b) = event {
            matching::reconcile(&mut tx, &BatchEvent::from(b)).await?;
        }
    }

    repo::set_cursor(&mut *tx, cursor_name, to as i64).await?;
    tx.commit().await?;
    if !events.is_empty() {
        tracing::info!(
            "indexed {} events in blocks {}..={}",
            events.len(),
            from,
            to
        );
    } else {
        tracing::debug!("no events in blocks {}..={}", from, to);
    }

    Ok(to == safe_head)
}

async fn decode_log(
    provider: &DynProvider,
    block_times: &mut HashMap<u64, i64>,
    chain_id: i64,
    log: &Log,
) -> Result<Option<ChainEvent>, BoxError> {
    let block_number = log.block_number.ok_or("log has no block number")?;

    let meta = LogMeta {
        chain_id,
        tx_hash: format!("{:#x}", log.transaction_hash.ok_or("log has no tx hash")?),
        log_index: log.log_index.ok_or("log has no index")? as i32,
        block_number: block_number as i64,
        block_time: block_time(provider, block_times, log, block_number).await?,
    };

    let event = match log.topic0() {
        Some(&BatchPayment::SIGNATURE_HASH) => {
            let e = log.log_decode::<BatchPayment>()?.inner.data;
            ChainEvent::Batch(OnchainBatch {
                meta,
                batch_ref: Uuid::from_bytes(e.batchRef.0),
                sender: format!("{:#x}", e.sender),
                token: format!("{:#x}", e.token),
                total_amount: e.totalAmount.to_string(),
                recipient_count: e.recipientCount.saturating_to::<i32>(),
            })
        }
        Some(&PaymentSent::SIGNATURE_HASH) => {
            let e = log.log_decode::<PaymentSent>()?.inner.data;
            ChainEvent::Payment(OnchainPayment {
                meta,
                batch_ref: Uuid::from_bytes(e.batchRef.0),
                sender: format!("{:#x}", e.sender),
                recipient: format!("{:#x}", e.recipient),
                token: format!("{:#x}", e.token),
                amount: e.amount.to_string(),
            })
        }
        _ => return Ok(None),
    };

    Ok(Some(event))
}

/// Some RPCs include the block time on the log; otherwise fetch the block once and reuse it.
/// A 200-recipient batch is 201 logs in the same block, so the cache matters.
async fn block_time(
    provider: &DynProvider,
    cache: &mut HashMap<u64, i64>,
    log: &Log,
    block_number: u64,
) -> Result<i64, BoxError> {
    if let Some(ts) = log.block_timestamp {
        return Ok(ts as i64);
    }
    if let Some(ts) = cache.get(&block_number) {
        return Ok(*ts);
    }

    let ts = provider
        .get_block_by_number(block_number.into())
        .await?
        .ok_or("block not found")?
        .header
        .timestamp as i64;

    cache.insert(block_number, ts);
    Ok(ts)
}
