use sqlx::{PgExecutor, prelude::FromRow};
use uuid::Uuid;

use crate::handlers::users::repo::UserRow;

#[derive(Debug, FromRow)]
pub struct OAuthStateRow {
    pub pkce_verifier: String,
    pub nonce: String,
}

#[derive(Debug, FromRow)]
pub struct RecentCodeCounts {
    pub last_minute: i64,
    pub last_hour: i64,
}

#[derive(Debug, FromRow)]
pub struct ActiveCode {
    pub id: Uuid,
    pub code_hash: String,
}

pub async fn count_recent_codes(
    db: impl PgExecutor<'_>,
    email: &str,
) -> Result<RecentCodeCounts, sqlx::Error> {
    sqlx::query_as::<_, RecentCodeCounts>(
        "SELECT
             COUNT(*) FILTER (WHERE created_at > NOW() - INTERVAL '60 seconds') AS last_minute,
             COUNT(*) AS last_hour
         FROM login_codes
         WHERE LOWER(email) = LOWER($1)
           AND created_at > NOW() - INTERVAL '1 hour'",
    )
    .bind(email)
    .fetch_one(db)
    .await
}

/// `code_hash` is the HMAC of the code, never the code itself.
pub async fn insert_code(
    db: impl PgExecutor<'_>,
    email: &str,
    code_hash: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO login_codes (email, code_hash, expires_at)
         VALUES (LOWER($1), $2, NOW() + INTERVAL '10 minutes')",
    )
    .bind(email)
    .bind(code_hash)
    .execute(db)
    .await?;

    Ok(())
}

/// Marks every unused code for this email as used, so only the next one works.
pub async fn consume_unused_codes(db: impl PgExecutor<'_>, email: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE login_codes
         SET consumed_at = NOW()
         WHERE LOWER(email) = LOWER($1)
           AND consumed_at IS NULL",
    )
    .bind(email)
    .execute(db)
    .await?;

    Ok(())
}

pub async fn delete_old_codes(db: impl PgExecutor<'_>) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("DELETE FROM login_codes WHERE created_at < NOW() - INTERVAL '1 day'")
        .execute(db)
        .await?;

    Ok(result.rows_affected())
}

pub async fn find_active_code(
    db: impl PgExecutor<'_>,
    email: &str,
) -> Result<Option<ActiveCode>, sqlx::Error> {
    sqlx::query_as::<_, ActiveCode>(
        "SELECT id, code_hash
         FROM login_codes
         WHERE LOWER(email) = LOWER($1)
           AND consumed_at IS NULL
           AND expires_at > NOW()
           AND attempts < 5
         ORDER BY created_at DESC
         LIMIT 1",
    )
    .bind(email)
    .fetch_optional(db)
    .await
}

/// Uses up one attempt before the code is checked. Returns false if the code is
/// out of attempts. Check and increment are one statement, so parallel guesses
/// can't all slip past the limit.
pub async fn use_attempt(db: impl PgExecutor<'_>, id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE login_codes
         SET attempts = attempts + 1
         WHERE id = $1 AND attempts < 5",
    )
    .bind(id)
    .execute(db)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// Returns false if it was already used (e.g. two requests at the same moment).
pub async fn consume_code(db: impl PgExecutor<'_>, id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE login_codes
         SET consumed_at = NOW()
         WHERE id = $1 AND consumed_at IS NULL",
    )
    .bind(id)
    .execute(db)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// Returns the existing user for this email, or creates one.
pub async fn find_or_create_user_by_email(
    db: impl PgExecutor<'_>,
    email: &str,
) -> Result<UserRow, sqlx::Error> {
    sqlx::query_as::<_, UserRow>(
        "INSERT INTO users (email)
         VALUES (LOWER($1))
         ON CONFLICT ((LOWER(email))) DO UPDATE SET email = users.email
         RETURNING id, full_name, email, google_sub, avatar_url, created_at, updated_at",
    )
    .bind(email)
    .fetch_one(db)
    .await
}

/// `jti` is the refresh token's `jti` claim; `exp` is its `exp` claim (Unix seconds).
pub async fn store_refresh_token(
    db: impl PgExecutor<'_>,
    user_id: Uuid,
    jti: Uuid,
    exp: usize,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO refresh_tokens (id, user_id, expires_at)
         VALUES ($1, $2, to_timestamp($3))",
    )
    .bind(jti)
    .bind(user_id)
    .bind(exp as i64)
    .execute(db)
    .await?;

    Ok(())
}

/// Revokes the refresh token if it is still live. Returns `None` if it was
/// already revoked, expired, or never existed (a sign the token was reused).
pub async fn revoke_refresh_token_if_valid(
    db: impl PgExecutor<'_>,
    user_id: Uuid,
    jti: Uuid,
) -> Result<Option<Uuid>, sqlx::Error> {
    sqlx::query_scalar::<_, Uuid>(
        "UPDATE refresh_tokens
         SET revoked_at = NOW()
         WHERE id = $1
           AND user_id = $2
           AND revoked_at IS NULL
           AND expires_at > NOW()
         RETURNING id",
    )
    .bind(jti)
    .bind(user_id)
    .fetch_optional(db)
    .await
}

/// Signs the user out everywhere.
pub async fn revoke_all_refresh_tokens(
    db: impl PgExecutor<'_>,
    user_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE refresh_tokens
         SET revoked_at = NOW()
         WHERE user_id = $1 AND revoked_at IS NULL",
    )
    .bind(user_id)
    .execute(db)
    .await?;

    Ok(())
}

/// Revokes one refresh token (one session). Already revoked is a no-op.
pub async fn revoke_refresh_token(
    db: impl PgExecutor<'_>,
    user_id: Uuid,
    jti: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE refresh_tokens
         SET revoked_at = NOW()
         WHERE id = $1
           AND user_id = $2
           AND revoked_at IS NULL",
    )
    .bind(jti)
    .bind(user_id)
    .execute(db)
    .await?;

    Ok(())
}

/// Saves the values needed to finish a Google sign-in. Valid for 10 minutes.
pub async fn insert_oauth_state(
    db: impl PgExecutor<'_>,
    state: &str,
    pkce_verifier: &str,
    nonce: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO oauth_states (state, pkce_verifier, nonce, expires_at)
         VALUES ($1, $2, $3, NOW() + INTERVAL '10 minutes')",
    )
    .bind(state)
    .bind(pkce_verifier)
    .bind(nonce)
    .execute(db)
    .await?;

    Ok(())
}

pub async fn delete_expired_oauth_states(db: impl PgExecutor<'_>) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("DELETE FROM oauth_states WHERE expires_at < NOW()")
        .execute(db)
        .await?;

    Ok(result.rows_affected())
}

/// Deletes and returns the saved state in one statement, so it can only be used once.
/// `None` if it never existed, was already used, or has expired.
pub async fn take_oauth_state(
    db: impl PgExecutor<'_>,
    state: &str,
) -> Result<Option<OAuthStateRow>, sqlx::Error> {
    sqlx::query_as::<_, OAuthStateRow>(
        "DELETE FROM oauth_states
         WHERE state = $1 AND expires_at > NOW()
         RETURNING pkce_verifier, nonce",
    )
    .bind(state)
    .fetch_optional(db)
    .await
}

pub async fn find_user_by_google_sub(
    db: impl PgExecutor<'_>,
    google_sub: &str,
) -> Result<Option<UserRow>, sqlx::Error> {
    sqlx::query_as::<_, UserRow>(
        "SELECT id, full_name, email, google_sub, avatar_url, created_at, updated_at
         FROM users
         WHERE google_sub = $1",
    )
    .bind(google_sub)
    .fetch_optional(db)
    .await
}

/// Links Google to an existing email account. Name and avatar are only filled
/// in if the user hasn't set them. `None` if no unlinked account has this email.
pub async fn link_google_to_email_user(
    db: impl PgExecutor<'_>,
    email: &str,
    google_sub: &str,
    full_name: Option<&str>,
    avatar_url: Option<&str>,
) -> Result<Option<UserRow>, sqlx::Error> {
    sqlx::query_as::<_, UserRow>(
        "UPDATE users
         SET google_sub = $2,
             full_name = COALESCE(full_name, $3),
             avatar_url = COALESCE(avatar_url, $4),
             updated_at = NOW()
         WHERE LOWER(email) = LOWER($1) AND google_sub IS NULL
         RETURNING id, full_name, email, google_sub, avatar_url, created_at, updated_at",
    )
    .bind(email)
    .bind(google_sub)
    .bind(full_name)
    .bind(avatar_url)
    .fetch_optional(db)
    .await
}

pub async fn create_google_user(
    db: impl PgExecutor<'_>,
    email: &str,
    google_sub: &str,
    full_name: Option<&str>,
    avatar_url: Option<&str>,
) -> Result<UserRow, sqlx::Error> {
    sqlx::query_as::<_, UserRow>(
        "INSERT INTO users (email, google_sub, full_name, avatar_url)
         VALUES (LOWER($1), $2, $3, $4)
         RETURNING id, full_name, email, google_sub, avatar_url, created_at, updated_at",
    )
    .bind(email)
    .bind(google_sub)
    .bind(full_name)
    .bind(avatar_url)
    .fetch_one(db)
    .await
}

/// `ticket_hash` is the HMAC of the ticket, never the ticket itself. Valid for 1 minute.
pub async fn insert_auth_ticket(
    db: impl PgExecutor<'_>,
    ticket_hash: &str,
    user_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO auth_tickets (ticket_hash, user_id, expires_at)
         VALUES ($1, $2, NOW() + INTERVAL '1 minute')",
    )
    .bind(ticket_hash)
    .bind(user_id)
    .execute(db)
    .await?;

    Ok(())
}

/// Marks the ticket used and returns its user. `None` if it doesn't exist, was
/// already used, or has expired. One statement, so a ticket can't be used twice.
pub async fn consume_auth_ticket(
    db: impl PgExecutor<'_>,
    ticket_hash: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    sqlx::query_scalar::<_, Uuid>(
        "UPDATE auth_tickets
         SET consumed_at = NOW()
         WHERE ticket_hash = $1
           AND consumed_at IS NULL
           AND expires_at > NOW()
         RETURNING user_id",
    )
    .bind(ticket_hash)
    .fetch_optional(db)
    .await
}

pub async fn delete_expired_auth_tickets(db: impl PgExecutor<'_>) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("DELETE FROM auth_tickets WHERE expires_at < NOW()")
        .execute(db)
        .await?;

    Ok(result.rows_affected())
}

/// Expired refresh tokens fail JWT decoding before the DB is checked, so their rows are dead.
pub async fn delete_expired_refresh_tokens(db: impl PgExecutor<'_>) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("DELETE FROM refresh_tokens WHERE expires_at < NOW()")
        .execute(db)
        .await?;

    Ok(result.rows_affected())
}
