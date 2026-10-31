use std::collections::HashSet;

use alloy::primitives::{Address, B256};
use chrono::Utc;
use uuid::Uuid;

use super::{
    dto::{
        CreatePayoutRequest, PayoutItemInput, PayoutResponse, SaveAsListRequest, SubmitBatchRequest,
    },
    repo::{self, NewPayout, NewPayoutItem},
};
use crate::{
    error::AppError,
    handlers::{
        lists::{
            dto::ListDetailResponse,
            repo::{self as lists_repo, NewRecipient},
        },
        payouts::dto::{PreparePayoutRequest, RenamePayoutRequest, UpdatePayoutItemsRequest},
    },
    state::AppState,
    utils::{clean_amount, clean_email, clean_name, clean_wallet},
};

pub async fn create_payout(
    state: &AppState,
    user_id: Uuid,
    req: CreatePayoutRequest,
) -> Result<PayoutResponse, AppError> {
    if req.chain_id != state.config.chain_id {
        return Err(AppError::BadRequest(
            "This network isn't supported".to_string(),
        ));
    }
    let token = req
        .token_address
        .trim()
        .parse::<Address>()
        .map_err(|_| AppError::BadRequest("Invalid token address".to_string()))?;
    if token != state.config.token_address {
        return Err(AppError::BadRequest(
            "Only USDC is supported for now".to_string(),
        ));
    }

    let items = clean_items(req.items)?;

    let name = req
        .name
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| format!("Payment {}", Utc::now().format("%-d %b %Y")));

    let token_address = format!("{token:#x}");

    // Payout and items together: never a payout with half its people.
    let mut tx = state.db.begin().await?;

    if let Some(list_id) = req.list_id {
        lists_repo::find_list_by_id(&mut *tx, user_id, list_id)
            .await?
            .ok_or_else(|| AppError::NotFound("List not found".to_string()))?;
    }

    let payout = repo::insert_payout(
        &mut *tx,
        NewPayout {
            user_id,
            list_id: req.list_id,
            name: &name,
            chain_id: state.config.chain_id,
            token_address: &token_address,
            token_symbol: &state.config.token_symbol,
            token_decimals: state.config.token_decimals,
        },
    )
    .await?;

    repo::insert_items(&mut *tx, payout.id, &items).await?;
    let items = repo::find_items_by_payout(&mut *tx, payout.id).await?;

    tx.commit().await?;

    Ok(PayoutResponse::new(payout, items, vec![]))
}

pub async fn get_payout(
    state: &AppState,
    user_id: Uuid,
    id: Uuid,
) -> Result<PayoutResponse, AppError> {
    // Ownership check first: someone else's payout looks like a missing one.
    let payout = repo::find_payout_by_id(&state.db, user_id, id)
        .await?
        .ok_or_else(|| AppError::NotFound("Payout not found".to_string()))?;

    let items = repo::find_items_by_payout(&state.db, payout.id).await?;
    let batches = repo::find_batches_by_payout(&state.db, payout.id).await?;

    Ok(PayoutResponse::new(payout, items, batches))
}

pub async fn update_payout_items(
    state: &AppState,
    user_id: Uuid,
    id: Uuid,
    req: UpdatePayoutItemsRequest,
) -> Result<PayoutResponse, AppError> {
    // Check every row before touching the DB.
    let items = clean_items(req.items)?;

    let mut tx = state.db.begin().await?;

    let payout = repo::find_payout_for_update(&mut *tx, user_id, id)
        .await?
        .ok_or_else(|| AppError::NotFound("Payout not found".to_string()))?;

    if payout.status != "draft" {
        return Err(AppError::Conflict(
            "Only draft payouts can be edited".to_string(),
        ));
    }

    // Replace, not merge: the frontend always sends the full list.
    repo::delete_items(&mut *tx, payout.id).await?;
    repo::insert_items(&mut *tx, payout.id, &items).await?;
    let payout = repo::touch_payout(&mut *tx, payout.id).await?;

    let items = repo::find_items_by_payout(&mut *tx, payout.id).await?;

    tx.commit().await?;

    Ok(PayoutResponse::new(payout, items, vec![]))
}

pub async fn rename_payout(
    state: &AppState,
    user_id: Uuid,
    id: Uuid,
    req: RenamePayoutRequest,
) -> Result<PayoutResponse, AppError> {
    let name = req.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("Name can't be empty".to_string()));
    }

    let payout = repo::rename_payout(&state.db, user_id, id, name)
        .await?
        .ok_or_else(|| AppError::NotFound("Payout not found".to_string()))?;

    let items = repo::find_items_by_payout(&state.db, payout.id).await?;
    let batches = repo::find_batches_by_payout(&state.db, payout.id).await?;

    Ok(PayoutResponse::new(payout, items, batches))
}

/// Works at any status: before sending, or from history.
pub async fn save_as_list(
    state: &AppState,
    user_id: Uuid,
    id: Uuid,
    req: SaveAsListRequest,
) -> Result<ListDetailResponse, AppError> {
    let name = req.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("Name can't be empty".to_string()));
    }

    let mut tx = state.db.begin().await?;

    let payout = repo::find_payout_for_update(&mut *tx, user_id, id)
        .await?
        .ok_or_else(|| AppError::NotFound("Payout not found".to_string()))?;

    let items = repo::find_items_by_payout(&mut *tx, payout.id).await?;

    // A list holds each wallet once, so a wallet paid twice keeps its first row.
    let mut seen = HashSet::new();
    let recipients: Vec<NewRecipient> = items
        .into_iter()
        .filter(|i| seen.insert(i.wallet_address.clone()))
        .map(|i| NewRecipient {
            name: i.name,
            email: i.email,
            wallet_address: i.wallet_address,
            default_amount: Some(i.amount),
        })
        .collect();

    let list = lists_repo::create_list(&mut *tx, user_id, name).await?;
    lists_repo::insert_recipients(&mut *tx, list.id, 0, &recipients).await?;
    repo::link_list(&mut *tx, payout.id, list.id).await?;

    let recipients = lists_repo::find_recipients_by_list(&mut *tx, list.id).await?;

    tx.commit().await?;

    Ok(ListDetailResponse::new(list, recipients))
}

const BATCH_SIZE: i32 = 200;

pub async fn prepare_payout(
    state: &AppState,
    user_id: Uuid,
    id: Uuid,
    req: PreparePayoutRequest,
) -> Result<PayoutResponse, AppError> {
    let payments = crate::feature_flags::get_flag(&state.db, "payments").await?;
    if !payments.enabled {
        return Err(AppError::ServiceUnavailable(
            payments
                .reason
                .unwrap_or_else(|| crate::feature_flags::PAYMENTS_PAUSED.to_string()),
        ));
    }

    let sender = req
        .sender_address
        .trim()
        .parse::<Address>()
        .map_err(|_| AppError::BadRequest("Invalid sender address".to_string()))?;
    if sender == Address::ZERO {
        return Err(AppError::BadRequest("Invalid sender address".to_string()));
    }
    let sender = format!("{sender:#x}");

    let mut tx = state.db.begin().await?;

    // Locked until commit, so an items edit or a second prepare has to wait.
    let payout = repo::find_payout_for_update(&mut *tx, user_id, id)
        .await?
        .ok_or_else(|| AppError::NotFound("Payout not found".to_string()))?;

    let payout = match payout.status.as_str() {
        "draft" => {
            repo::create_batches(&mut *tx, payout.id, BATCH_SIZE).await?;
            repo::assign_items_to_batches(&mut *tx, payout.id, BATCH_SIZE).await?;
            repo::mark_payout_ready(&mut *tx, payout.id, &sender).await?
        }
        // Clicking Send twice with the same wallet: same batches, same refs.
        "ready" if payout.sender_address.as_deref() == Some(sender.as_str()) => payout,
        "ready" => {
            return Err(AppError::Conflict(
                "This payout is prepared for another wallet".to_string(),
            ));
        }
        "cancelled" => {
            return Err(AppError::Conflict("This payout was cancelled".to_string()));
        }
        _ => {
            return Err(AppError::Conflict(
                "This payout has already been sent".to_string(),
            ));
        }
    };

    let items = repo::find_items_by_payout(&mut *tx, payout.id).await?;
    let batches = repo::find_batches_by_payout(&mut *tx, payout.id).await?;

    tx.commit().await?;

    Ok(PayoutResponse::new(payout, items, batches))
}

/// Called by the frontend as soon as the wallet returns a hash, so the payment
/// shows up before the indexer confirms it. Not gated by the breaker: the money already left.
pub async fn submit_batch(
    state: &AppState,
    user_id: Uuid,
    payout_id: Uuid,
    batch_id: Uuid,
    req: SubmitBatchRequest,
) -> Result<PayoutResponse, AppError> {
    let tx_hash = req.tx_hash.trim();
    let tx_hash = tx_hash
        .starts_with("0x")
        .then(|| tx_hash.parse::<B256>().ok())
        .flatten()
        .filter(|h| *h != B256::ZERO)
        .ok_or_else(|| AppError::BadRequest("Invalid transaction hash".to_string()))?;
    let tx_hash = format!("{tx_hash:#x}");

    let mut tx = state.db.begin().await?;

    let batch = repo::find_batch_for_update(&mut *tx, user_id, payout_id, batch_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Batch not found".to_string()))?;

    let unchanged = batch.status == "confirmed"
        || (batch.status == "submitted" && batch.tx_hash.as_deref() == Some(tx_hash.as_str()));

    if !unchanged {
        if batch.status == "dropped" {
            return Err(AppError::Conflict("This batch was dropped".to_string()));
        }
        if batch.payout_status == "cancelled" {
            return Err(AppError::Conflict("This payout was cancelled".to_string()));
        }

        repo::mark_batch_submitted(&mut *tx, batch_id, &tx_hash).await?;
        repo::mark_payout_submitted(&mut *tx, payout_id).await?;
    }

    let payout = repo::find_payout_by_id(&mut *tx, user_id, payout_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Payout not found".to_string()))?;
    let items = repo::find_items_by_payout(&mut *tx, payout_id).await?;
    let batches = repo::find_batches_by_payout(&mut *tx, payout_id).await?;

    tx.commit().await?;

    Ok(PayoutResponse::new(payout, items, batches))
}

fn clean_items(items: Vec<PayoutItemInput>) -> Result<Vec<NewPayoutItem>, AppError> {
    items
        .into_iter()
        .enumerate()
        .map(|(i, item)| {
            let row = i + 1;

            Ok(NewPayoutItem {
                wallet_address: clean_wallet(row, &item.wallet_address)?,
                amount: clean_amount(row, &item.amount)?,
                email: clean_email(row, item.email)?,
                name: clean_name(row, item.name)?,
            })
        })
        .collect()
}
