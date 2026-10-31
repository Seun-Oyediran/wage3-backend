use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::handlers::payouts::repo::PayoutBatchRow;

use super::repo::{PayoutItemRow, PayoutRow};

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct CreatePayoutRequest {
    #[validate(length(max = 100, message = "Name must be at most 100 characters"))]
    pub name: Option<String>,
    pub list_id: Option<Uuid>,
    pub chain_id: i64,
    pub token_address: String,
    #[validate(length(
        min = 1,
        max = 1000,
        message = "A payout needs between 1 and 1000 people"
    ))]
    pub items: Vec<PayoutItemInput>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PayoutItemInput {
    pub name: Option<String>,
    pub email: Option<String>,
    pub wallet_address: String,
    pub amount: String, // base units: "10000" = 0.01 USDC
}

#[derive(Debug, Serialize)]
pub struct PayoutItemResponse {
    pub id: Uuid,
    pub batch_id: Option<Uuid>,
    pub position: i32,
    pub name: Option<String>,
    pub email: Option<String>,
    pub wallet_address: String,
    pub amount: String,
    pub status: &'static str, // pending, sending, paid or dropped
    pub tx_hash: Option<String>,
    pub paid_at: Option<DateTime<Utc>>,
}

impl PayoutItemResponse {
    /// A person is paid when their batch is: a batch succeeds or fails as a whole.
    fn new(row: PayoutItemRow, batch: Option<&PayoutBatchResponse>) -> Self {
        let status = match batch.map(|b| b.status.as_str()) {
            Some("submitted") => "sending",
            Some("confirmed") => "paid",
            Some("dropped") => "dropped",
            _ => "pending",
        };

        Self {
            id: row.id,
            batch_id: row.batch_id,
            position: row.position,
            name: row.name,
            email: row.email,
            wallet_address: row.wallet_address,
            amount: row.amount,
            status,
            tx_hash: batch.and_then(|b| b.tx_hash.clone()),
            paid_at: batch.and_then(|b| b.confirmed_at),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PayoutResponse {
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
    pub batch_count: usize,
    pub confirmed_batch_count: usize,
    pub prepared_at: Option<DateTime<Utc>>,
    pub paid_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub items: Vec<PayoutItemResponse>,
    pub batches: Vec<PayoutBatchResponse>,
}

impl PayoutResponse {
    pub fn new(payout: PayoutRow, items: Vec<PayoutItemRow>, batches: Vec<PayoutBatchRow>) -> Self {
        let batches: Vec<PayoutBatchResponse> = batches
            .into_iter()
            .map(|b| {
                // Only a batch still to send needs them; sent ones are listed in `items`.
                let (recipients, amounts) = if b.status == "ready" {
                    let (r, a) = items
                        .iter()
                        .filter(|i| i.batch_id == Some(b.id))
                        .map(|i| (i.wallet_address.clone(), i.amount.clone()))
                        .unzip();
                    (Some(r), Some(a))
                } else {
                    (None, None)
                };

                PayoutBatchResponse {
                    id: b.id,
                    batch_ref: format!("0x{}", b.id.simple()),
                    batch_index: b.batch_index,
                    total_amount: b.total_amount,
                    recipient_count: b.recipient_count,
                    status: b.status,
                    tx_hash: b.tx_hash,
                    block_number: b.block_number,
                    submitted_at: b.submitted_at,
                    confirmed_at: b.confirmed_at,
                    recipients,
                    amounts,
                }
            })
            .collect();

        let by_id: HashMap<Uuid, &PayoutBatchResponse> =
            batches.iter().map(|b| (b.id, b)).collect();
        let items = items
            .into_iter()
            .map(|i| {
                let batch = i.batch_id.and_then(|id| by_id.get(&id).copied());
                PayoutItemResponse::new(i, batch)
            })
            .collect();

        Self {
            id: payout.id,
            list_id: payout.list_id,
            name: payout.name,
            chain_id: payout.chain_id,
            token_address: payout.token_address,
            token_symbol: payout.token_symbol,
            token_decimals: payout.token_decimals,
            sender_address: payout.sender_address,
            status: payout.status,
            total_amount: payout.total_amount,
            recipient_count: payout.recipient_count,
            batch_count: batches.len(),
            confirmed_batch_count: batches.iter().filter(|b| b.status == "confirmed").count(),
            prepared_at: payout.prepared_at,
            paid_at: payout.paid_at,
            created_at: payout.created_at,
            updated_at: payout.updated_at,
            items,
            batches,
        }
    }
}

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct UpdatePayoutItemsRequest {
    #[validate(length(
        min = 1,
        max = 1000,
        message = "A payout needs between 1 and 1000 people"
    ))]
    pub items: Vec<PayoutItemInput>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct RenamePayoutRequest {
    #[validate(length(
        min = 1,
        max = 100,
        message = "Name must be between 1 and 100 characters"
    ))]
    pub name: String,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct SaveAsListRequest {
    #[validate(length(
        min = 1,
        max = 100,
        message = "Name must be between 1 and 100 characters"
    ))]
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparePayoutRequest {
    pub sender_address: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubmitBatchRequest {
    pub tx_hash: String,
}

/// Everything the frontend needs for one batchTransfer call.
#[derive(Debug, Serialize)]
pub struct PayoutBatchResponse {
    pub id: Uuid,
    #[serde(rename = "ref")]
    pub batch_ref: String, // "0x" + 32 hex chars: the bytes16 for the contract
    pub batch_index: i32,
    pub total_amount: String,
    pub recipient_count: i32,
    pub status: String,
    pub tx_hash: Option<String>,
    pub block_number: Option<i64>,
    pub submitted_at: Option<DateTime<Utc>>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub recipients: Option<Vec<String>>, // same order as amounts, ready for batchTransfer
    pub amounts: Option<Vec<String>>,
}
