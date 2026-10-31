//! SQL for the `templates` table. No business rules here.
//!
//! Functions take `impl PgExecutor` so the caller can pass `&PgPool` or
//! `&mut *tx` and run several of them in one transaction.
//!
//! There is no `templates` migration on purpose (this is a sample). To try it:
//!
//! CREATE TABLE templates (
//!     id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
//!     name TEXT NOT NULL,
//!     description TEXT,
//!     created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
//!     updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
//! );
//!
//! In a real feature, use `sqlx::query_as!` so queries are checked against the
//! schema at compile time.

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgExecutor};
use uuid::Uuid;

#[derive(Debug, FromRow)]
pub struct TemplateRow {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub async fn insert(
    db: impl PgExecutor<'_>,
    name: &str,
    description: Option<&str>,
) -> Result<TemplateRow, sqlx::Error> {
    sqlx::query_as::<_, TemplateRow>(
        "INSERT INTO templates (name, description)
         VALUES ($1, $2)
         RETURNING id, name, description, created_at, updated_at",
    )
    .bind(name)
    .bind(description)
    .fetch_one(db)
    .await
}

pub async fn find_by_id(
    db: impl PgExecutor<'_>,
    id: Uuid,
) -> Result<Option<TemplateRow>, sqlx::Error> {
    sqlx::query_as::<_, TemplateRow>(
        "SELECT id, name, description, created_at, updated_at
         FROM templates
         WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(db)
    .await
}

pub async fn list(
    db: impl PgExecutor<'_>,
    limit: i64,
    offset: i64,
) -> Result<Vec<TemplateRow>, sqlx::Error> {
    sqlx::query_as::<_, TemplateRow>(
        "SELECT id, name, description, created_at, updated_at
         FROM templates
         ORDER BY created_at DESC, id
         LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await
}

pub async fn count(db: impl PgExecutor<'_>) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("SELECT COUNT(*) FROM templates")
        .fetch_one(db)
        .await
}

/// `None` fields keep their current value.
pub async fn update(
    db: impl PgExecutor<'_>,
    id: Uuid,
    name: Option<&str>,
    description: Option<&str>,
) -> Result<Option<TemplateRow>, sqlx::Error> {
    sqlx::query_as::<_, TemplateRow>(
        "UPDATE templates
         SET name = COALESCE($2, name),
             description = COALESCE($3, description),
             updated_at = NOW()
         WHERE id = $1
         RETURNING id, name, description, created_at, updated_at",
    )
    .bind(id)
    .bind(name)
    .bind(description)
    .fetch_optional(db)
    .await
}

/// Returns true if a row was deleted.
pub async fn delete(db: impl PgExecutor<'_>, id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM templates WHERE id = $1")
        .bind(id)
        .execute(db)
        .await?;

    Ok(result.rows_affected() > 0)
}
