use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use super::{
    dto::{
        AddRecipientsRequest, CreateListRequest, ListDetailResponse, ListResponse, RecipientInput,
        RenameListRequest, ReplaceRecipientsRequest,
    },
    repo::{self, NewRecipient},
};
use crate::{
    error::AppError,
    state::AppState,
    utils::{clean_amount, clean_email, clean_name, clean_wallet, row_error},
};

const MAX_RECIPIENTS: usize = 1000;

pub async fn create_list(
    state: &AppState,
    user_id: Uuid,
    req: CreateListRequest,
) -> Result<ListDetailResponse, AppError> {
    let name = clean_list_name(&req.name)?;
    let recipients = clean_recipients(req.recipients)?;

    let mut tx = state.db.begin().await?;

    let list = repo::create_list(&mut *tx, user_id, name).await?;
    repo::insert_recipients(&mut *tx, list.id, 0, &recipients).await?;
    let recipients = repo::find_recipients_by_list(&mut *tx, list.id).await?;

    tx.commit().await?;

    Ok(ListDetailResponse::new(list, recipients))
}

pub async fn get_lists(
    state: &AppState,
    user_id: Uuid,
    search: Option<&str>,
) -> Result<Vec<ListResponse>, AppError> {
    // `?search=` or `?search=  ` means no search.
    let search = search.map(str::trim).filter(|s| !s.is_empty());

    let lists = repo::find_lists(&state.db, user_id, search).await?;

    Ok(lists.into_iter().map(Into::into).collect())
}

pub async fn get_list(
    state: &AppState,
    user_id: Uuid,
    id: Uuid,
) -> Result<ListDetailResponse, AppError> {
    // Ownership check first: someone else's list looks like a missing one.
    let list = repo::find_list_by_id(&state.db, user_id, id)
        .await?
        .ok_or_else(|| AppError::NotFound("List not found".to_string()))?;

    let recipients = repo::find_recipients_by_list(&state.db, list.id).await?;

    Ok(ListDetailResponse::new(list, recipients))
}

pub async fn rename_list(
    state: &AppState,
    user_id: Uuid,
    id: Uuid,
    req: RenameListRequest,
) -> Result<ListDetailResponse, AppError> {
    let name = clean_list_name(&req.name)?;

    let list = repo::rename_list(&state.db, user_id, id, name)
        .await?
        .ok_or_else(|| AppError::NotFound("List not found".to_string()))?;

    let recipients = repo::find_recipients_by_list(&state.db, list.id).await?;

    Ok(ListDetailResponse::new(list, recipients))
}

pub async fn delete_list(state: &AppState, user_id: Uuid, id: Uuid) -> Result<(), AppError> {
    if !repo::delete_list(&state.db, user_id, id).await? {
        return Err(AppError::NotFound("List not found".to_string()));
    }

    Ok(())
}

pub async fn add_recipients(
    state: &AppState,
    user_id: Uuid,
    id: Uuid,
    req: AddRecipientsRequest,
) -> Result<ListDetailResponse, AppError> {
    let new = clean_recipients(req.recipients)?;

    let mut tx = state.db.begin().await?;

    repo::find_list_for_update(&mut *tx, user_id, id)
        .await?
        .ok_or_else(|| AppError::NotFound("List not found".to_string()))?;

    let existing = repo::find_recipients_by_list(&mut *tx, id).await?;

    if existing.len() + new.len() > MAX_RECIPIENTS {
        return Err(AppError::BadRequest(format!(
            "A list can have at most {MAX_RECIPIENTS} people. This one has room for {} more",
            MAX_RECIPIENTS - existing.len()
        )));
    }

    let existing_wallets: HashSet<&str> =
        existing.iter().map(|r| r.wallet_address.as_str()).collect();
    if let Some(i) = new
        .iter()
        .position(|r| existing_wallets.contains(r.wallet_address.as_str()))
    {
        return Err(row_error(i + 1, "this wallet is already in the list"));
    }

    let start_position = existing.last().map_or(0, |r| r.position + 1);
    repo::insert_recipients(&mut *tx, id, start_position, &new).await?;

    let list = repo::touch_list(&mut *tx, id).await?;
    let recipients = repo::find_recipients_by_list(&mut *tx, id).await?;

    tx.commit().await?;

    Ok(ListDetailResponse::new(list, recipients))
}

pub async fn replace_recipients(
    state: &AppState,
    user_id: Uuid,
    id: Uuid,
    req: ReplaceRecipientsRequest,
) -> Result<ListDetailResponse, AppError> {
    let recipients = clean_recipients(req.recipients)?;

    let mut tx = state.db.begin().await?;

    repo::find_list_for_update(&mut *tx, user_id, id)
        .await?
        .ok_or_else(|| AppError::NotFound("List not found".to_string()))?;

    repo::delete_recipients(&mut *tx, id).await?;
    repo::insert_recipients(&mut *tx, id, 0, &recipients).await?;

    let list = repo::touch_list(&mut *tx, id).await?;
    let recipients = repo::find_recipients_by_list(&mut *tx, id).await?;

    tx.commit().await?;

    Ok(ListDetailResponse::new(list, recipients))
}

/// "   " passes the length check but is really empty.
fn clean_list_name(name: &str) -> Result<&str, AppError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("Name can't be empty".to_string()));
    }

    Ok(name)
}

/// Checks every row and rejects a wallet that appears twice.
fn clean_recipients(rows: Vec<RecipientInput>) -> Result<Vec<NewRecipient>, AppError> {
    let mut seen: HashMap<String, usize> = HashMap::new();

    rows.into_iter()
        .enumerate()
        .map(|(i, r)| {
            let row = i + 1;

            let wallet_address = clean_wallet(row, &r.wallet_address)?;
            if let Some(first) = seen.get(&wallet_address) {
                return Err(row_error(
                    row,
                    &format!("this wallet is already in the list (row {first})"),
                ));
            }
            seen.insert(wallet_address.clone(), row);

            let default_amount = r
                .default_amount
                .filter(|a| !a.trim().is_empty())
                .map(|a| clean_amount(row, &a))
                .transpose()?;

            Ok(NewRecipient {
                wallet_address,
                default_amount,
                email: clean_email(row, r.email)?,
                name: clean_name(row, r.name)?,
            })
        })
        .collect()
}
