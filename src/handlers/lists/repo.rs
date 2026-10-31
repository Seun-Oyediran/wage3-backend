use chrono::{DateTime, Utc};
use sqlx::{PgExecutor, prelude::FromRow};
use uuid::Uuid;

#[derive(Debug, FromRow)]
pub struct ListRow {
    pub id: Uuid,
    #[allow(dead_code)]
    pub user_id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
pub struct ListSummaryRow {
    pub id: Uuid,
    pub name: String,
    pub recipient_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
pub struct RecipientRow {
    pub id: Uuid,
    pub position: i32,
    pub name: Option<String>,
    pub email: Option<String>,
    pub wallet_address: String,
    pub default_amount: Option<String>,
}

/// A checked, cleaned row, ready to insert.
#[derive(Debug)]
pub struct NewRecipient {
    pub name: Option<String>,
    pub email: Option<String>,
    pub wallet_address: String,
    pub default_amount: Option<String>,
}

pub async fn create_list(
    db: impl PgExecutor<'_>,
    user_id: Uuid,
    name: &str,
) -> Result<ListRow, sqlx::Error> {
    sqlx::query_as::<_, ListRow>(
        "INSERT INTO lists (user_id, name)
         VALUES ($1, $2)
         RETURNING id, user_id, name, created_at, updated_at",
    )
    .bind(user_id)
    .bind(name)
    .fetch_one(db)
    .await
}

pub async fn find_lists(
    db: impl PgExecutor<'_>,
    user_id: Uuid,
    search: Option<&str>,
) -> Result<Vec<ListSummaryRow>, sqlx::Error> {
    sqlx::query_as::<_, ListSummaryRow>(
        "SELECT l.id, l.name, COUNT(r.id) AS recipient_count, l.created_at, l.updated_at
         FROM lists l
         LEFT JOIN recipients r ON r.list_id = l.id
         WHERE l.user_id = $1
           AND ($2::TEXT IS NULL OR l.name ILIKE '%' || $2 || '%')
         GROUP BY l.id
         ORDER BY l.created_at DESC",
    )
    .bind(user_id)
    .bind(search)
    .fetch_all(db)
    .await
}

/// One list, only if it belongs to this user.
pub async fn find_list_by_id(
    db: impl PgExecutor<'_>,
    user_id: Uuid,
    id: Uuid,
) -> Result<Option<ListRow>, sqlx::Error> {
    sqlx::query_as::<_, ListRow>(
        "SELECT id, user_id, name, created_at, updated_at
         FROM lists
         WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(db)
    .await
}

/// Same as find_list_by_id, but locks the row until the transaction ends,
/// so two edits to the same list's people run one after the other.
pub async fn find_list_for_update(
    db: impl PgExecutor<'_>,
    user_id: Uuid,
    id: Uuid,
) -> Result<Option<ListRow>, sqlx::Error> {
    sqlx::query_as::<_, ListRow>(
        "SELECT id, user_id, name, created_at, updated_at
         FROM lists
         WHERE id = $1 AND user_id = $2
         FOR UPDATE",
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(db)
    .await
}

pub async fn touch_list(db: impl PgExecutor<'_>, id: Uuid) -> Result<ListRow, sqlx::Error> {
    sqlx::query_as::<_, ListRow>(
        "UPDATE lists
         SET updated_at = NOW()
         WHERE id = $1
         RETURNING id, user_id, name, created_at, updated_at",
    )
    .bind(id)
    .fetch_one(db)
    .await
}

pub async fn rename_list(
    db: impl PgExecutor<'_>,
    user_id: Uuid,
    id: Uuid,
    name: &str,
) -> Result<Option<ListRow>, sqlx::Error> {
    sqlx::query_as::<_, ListRow>(
        "UPDATE lists
         SET name = $3, updated_at = NOW()
         WHERE id = $1 AND user_id = $2
         RETURNING id, user_id, name, created_at, updated_at",
    )
    .bind(id)
    .bind(user_id)
    .bind(name)
    .fetch_optional(db)
    .await
}

/// Its recipients go with it (cascade); payouts made from it are kept.
pub async fn delete_list(
    db: impl PgExecutor<'_>,
    user_id: Uuid,
    id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM lists WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user_id)
        .execute(db)
        .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn delete_recipients(db: impl PgExecutor<'_>, list_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM recipients WHERE list_id = $1")
        .bind(list_id)
        .execute(db)
        .await?;

    Ok(())
}

/// Inserts after `start_position`, in the order given. UNNEST zips the column arrays into rows.
pub async fn insert_recipients(
    db: impl PgExecutor<'_>,
    list_id: Uuid,
    start_position: i32,
    recipients: &[NewRecipient],
) -> Result<(), sqlx::Error> {
    let positions: Vec<i32> = (0..recipients.len() as i32)
        .map(|i| start_position + i)
        .collect();
    let names: Vec<Option<String>> = recipients.iter().map(|r| r.name.clone()).collect();
    let emails: Vec<Option<String>> = recipients.iter().map(|r| r.email.clone()).collect();
    let wallets: Vec<String> = recipients
        .iter()
        .map(|r| r.wallet_address.clone())
        .collect();
    let amounts: Vec<Option<String>> = recipients
        .iter()
        .map(|r| r.default_amount.clone())
        .collect();

    sqlx::query(
        "INSERT INTO recipients (list_id, position, name, email, wallet_address, default_amount)
         SELECT $1, t.position, t.name, t.email, t.wallet_address, t.default_amount::NUMERIC
         FROM UNNEST($2::INT[], $3::TEXT[], $4::TEXT[], $5::TEXT[], $6::TEXT[])
              AS t(position, name, email, wallet_address, default_amount)",
    )
    .bind(list_id)
    .bind(positions)
    .bind(names)
    .bind(emails)
    .bind(wallets)
    .bind(amounts)
    .execute(db)
    .await?;

    Ok(())
}

pub async fn find_recipients_by_list(
    db: impl PgExecutor<'_>,
    list_id: Uuid,
) -> Result<Vec<RecipientRow>, sqlx::Error> {
    sqlx::query_as::<_, RecipientRow>(
        "SELECT id, position, name, email, wallet_address,
                default_amount::TEXT AS default_amount
         FROM recipients
         WHERE list_id = $1
         ORDER BY position",
    )
    .bind(list_id)
    .fetch_all(db)
    .await
}
