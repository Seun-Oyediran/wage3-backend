use uuid::Uuid;

use super::{
    dto::{StatsQuery, StatsResponse, UserResponse},
    repo,
};
use crate::{error::AppError, state::AppState, utils::date_range};

pub async fn get_me(state: &AppState, user_id: Uuid) -> Result<UserResponse, AppError> {
    repo::find_by_id(&state.db, user_id)
        .await?
        .map(Into::into)
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))
}

pub async fn get_stats(
    state: &AppState,
    user_id: Uuid,
    query: StatsQuery,
) -> Result<StatsResponse, AppError> {
    let (from, to) = date_range(query.from, query.to)?;

    // One snapshot, so the totals and the counts agree.
    let mut tx = state.db.begin().await?;
    let totals = repo::sum_sent_by_token(&mut *tx, user_id, from, to).await?;
    let counts = repo::count_sent(&mut *tx, user_id, from, to).await?;
    tx.commit().await?;

    Ok(StatsResponse::new(totals, counts))
}
