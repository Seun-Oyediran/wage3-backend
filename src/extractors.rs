use axum::extract::FromRequest;
use axum::extract::FromRequestParts;

use crate::error::AppError;

// Same as axum's extractors, but rejections go through AppError so clients
// always get the `{ message, success: false }` shape.

#[derive(FromRequest)]
#[from_request(via(axum::Json), rejection(AppError))]
pub struct AppJson<T>(pub T);

#[derive(FromRequestParts)]
#[from_request(via(axum::extract::Query), rejection(AppError))]
pub struct AppQuery<T>(pub T);

#[derive(FromRequestParts)]
#[from_request(via(axum::extract::Path), rejection(AppError))]
pub struct AppPath<T>(pub T);
