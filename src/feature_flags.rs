use sqlx::{FromRow, PgExecutor};

/// Shown when payments are off and no reason was set.
pub const PAYMENTS_PAUSED: &str = "Payments are paused. Please try again later";

#[derive(Debug, FromRow)]
pub struct FeatureFlag {
    pub enabled: bool,
    pub reason: Option<String>,
}

/// A missing flag counts as disabled, so a typo in a key fails safe.
pub async fn get_flag(db: impl PgExecutor<'_>, key: &str) -> Result<FeatureFlag, sqlx::Error> {
    let flag = sqlx::query_as::<_, FeatureFlag>(
        "SELECT enabled, reason FROM feature_flags WHERE key = $1",
    )
    .bind(key)
    .fetch_optional(db)
    .await?;

    Ok(flag.unwrap_or(FeatureFlag {
        enabled: false,
        reason: None,
    }))
}
