use axum::{Json, http::StatusCode};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::error::AppError;

/// Return type for handlers that reply 200 with the success envelope.
pub type ApiResult<T> = Result<Json<ApiResponse<T>>, AppError>;

/// Return type for handlers that pick their own status code (e.g. 201 Created).
pub type ApiResultWithStatus<T> = Result<(StatusCode, Json<ApiResponse<T>>), AppError>;

/// Success envelope. Mirrors `ErrorBody` in error.rs so every response has a `success` field.
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    pub data: Option<T>,
    pub message: String,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn new(data: Option<T>, message: &str) -> Self {
        Self {
            success: true,
            data,
            message: message.to_string(),
        }
    }
}

const DEFAULT_PAGE_LIMIT: i64 = 20;
const MAX_PAGE_LIMIT: i64 = 100;

#[derive(Debug, Deserialize, Validate)]
pub struct PaginatedRequest {
    #[validate(range(min = 1, message = "page must be greater than 0"))]
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

impl PaginatedRequest {
    pub fn page(&self) -> i64 {
        self.page.unwrap_or(1)
    }

    /// Out of range is corrected, not rejected: 500 -> 100, 0 -> 1.
    pub fn limit(&self) -> i64 {
        self.limit
            .unwrap_or(DEFAULT_PAGE_LIMIT)
            .clamp(1, MAX_PAGE_LIMIT)
    }

    /// Rows to skip. Saturates so a huge `page` returns an empty page instead of overflowing.
    pub fn offset(&self) -> i64 {
        (self.page() - 1).saturating_mul(self.limit())
    }
}

#[derive(Debug, Serialize)]
pub struct PaginatedResponse {
    total_items: i64,
    total_pages: i64,
    current_page: i64,
    limit: i64,
    has_next_page: bool,
    has_previous_page: bool,
}

impl PaginatedResponse {
    pub fn new(total: i64, limit: i64, page: i64) -> Self {
        let mut total_pages = (total + (limit - 1)) / limit;
        total_pages = total_pages.max(1);

        Self {
            total_items: total,
            total_pages,
            current_page: page,
            limit,
            has_next_page: page < total_pages,
            has_previous_page: page > 1,
        }
    }
}

/// `data` for list endpoints: the page of items plus the pagination info.
#[derive(Debug, Serialize)]
pub struct PaginatedList<T: Serialize> {
    pub items: Vec<T>,
    pub pagination: PaginatedResponse,
}
