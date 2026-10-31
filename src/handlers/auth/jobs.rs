use std::time::Duration;

use tokio::time::interval;

use sqlx::PgPool;

use super::repo::{
    delete_expired_auth_tickets, delete_expired_oauth_states, delete_expired_refresh_tokens,
    delete_old_codes,
};

pub fn spawn_cleanup_job(db: PgPool) {
    tokio::spawn(async move {
        let mut cleanup_interval = interval(Duration::from_secs(60 * 60));
        loop {
            cleanup_interval.tick().await;

            match delete_old_codes(&db).await {
                Err(e) => {
                    tracing::error!("Failed to delete old login codes: {}", e);
                }
                Ok(count) => {
                    tracing::debug!("deleted {} old login codes", count);
                }
            }

            match delete_expired_oauth_states(&db).await {
                Err(e) => {
                    tracing::error!("Failed to delete expired oauth states: {}", e);
                }
                Ok(count) => {
                    tracing::debug!("deleted {} expired oauth states", count);
                }
            }

            match delete_expired_auth_tickets(&db).await {
                Err(e) => {
                    tracing::error!("Failed to delete expired auth tickets: {}", e);
                }
                Ok(count) => {
                    tracing::debug!("deleted {} expired auth tickets", count);
                }
            }

            match delete_expired_refresh_tokens(&db).await {
                Err(e) => {
                    tracing::error!("Failed to delete expired refresh tokens: {}", e);
                }
                Ok(count) => {
                    tracing::debug!("deleted {} expired refresh tokens", count);
                }
            }
        }
    });
}
