use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

use super::{
    dto::{
        AdminStatsQuery, AdminStatsResponse, AppStatsResponse, Health, PayoutStatusCounts,
        UserStats,
    },
    repo,
};
use crate::{error::AppError, feature_flags, state::AppState, utils::date_range};

/// Public and unauthenticated, and every call scans all payouts: serve a copy for a minute.
const CACHE_FOR: Duration = Duration::from_secs(60);
static APP_STATS: Mutex<Option<(Instant, AppStatsResponse)>> = Mutex::new(None);

pub async fn get_app_stats(state: &AppState) -> Result<AppStatsResponse, AppError> {
    if let Some((at, stats)) = APP_STATS.lock().unwrap().as_ref()
        && at.elapsed() < CACHE_FOR
    {
        return Ok(stats.clone());
    }

    let stats = load_volume(state, None, None).await?;
    *APP_STATS.lock().unwrap() = Some((Instant::now(), stats.clone()));

    Ok(stats)
}

pub async fn get_admin_stats(
    state: &AppState,
    query: AdminStatsQuery,
) -> Result<AdminStatsResponse, AppError> {
    let (from, to) = date_range(query.from, query.to)?;
    let cursor_name = format!(
        "batcher:{}:{:#x}",
        state.config.chain_id, state.config.batcher_address
    );

    let volume = load_volume(state, from, to).await?;
    let statuses = repo::count_payouts_by_status(&state.db).await?;
    let health = repo::find_health(&state.db, from, to, &cursor_name).await?;
    let payments = feature_flags::get_flag(&state.db, "payments").await?;

    Ok(AdminStatsResponse {
        volume,
        users: UserStats {
            total: health.users_total,
            new: health.users_new,
            active_senders: health.active_senders,
        },
        payouts_by_status: PayoutStatusCounts::new(statuses),
        health: Health::new(&health, payments.enabled),
    })
}

async fn load_volume(
    state: &AppState,
    from: Option<chrono::DateTime<chrono::Utc>>,
    to: Option<chrono::DateTime<chrono::Utc>>,
) -> Result<AppStatsResponse, AppError> {
    // One snapshot, so the totals and the counts agree.
    let mut tx = state.db.begin().await?;
    let totals = repo::sum_sent_by_token(&mut *tx, from, to).await?;
    let counts = repo::count_sent(&mut *tx, from, to).await?;
    tx.commit().await?;

    Ok(AppStatsResponse::new(totals, counts))
}
