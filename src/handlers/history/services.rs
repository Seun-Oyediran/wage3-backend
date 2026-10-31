use uuid::Uuid;

use super::{
    dto::{SentHistoryQuery, SentPayoutResponse},
    repo::{self, SentFilters},
};
use crate::{
    error::AppError,
    response::{PaginatedList, PaginatedRequest, PaginatedResponse},
    state::AppState,
    utils::date_range,
};

const SENT_STATUSES: [&str; 3] = ["submitted", "paid", "partially_paid"];

pub async fn get_sent(
    state: &AppState,
    user_id: Uuid,
    query: SentHistoryQuery,
    paging: &PaginatedRequest,
) -> Result<PaginatedList<SentPayoutResponse>, AppError> {
    let status = query
        .status
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    if let Some(s) = status
        && !SENT_STATUSES.contains(&s)
    {
        return Err(AppError::BadRequest(
            "status must be submitted, paid or partially_paid".to_string(),
        ));
    }

    let (from, to) = date_range(query.from, query.to)?;

    let filters = SentFilters {
        user_id,
        status,
        list_id: query.list_id,
        search: query
            .search
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty()),
        from,
        to,
    };
    let (page, limit) = (paging.page(), paging.limit());

    // Both reads in one transaction so `total` matches the page.
    let mut tx = state.db.begin().await?;
    let rows = repo::find_sent(&mut *tx, &filters, limit, paging.offset()).await?;
    let total = repo::count_sent(&mut *tx, &filters).await?;
    tx.commit().await?;

    Ok(PaginatedList {
        items: rows.into_iter().map(Into::into).collect(),
        pagination: PaginatedResponse::new(total, limit, page),
    })
}
