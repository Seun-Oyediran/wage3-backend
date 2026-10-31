use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::Redirect,
};
use openidconnect::url::Url;
use validator::Validate;

use crate::{
    error::AppError,
    extractors::AppJson,
    response::{ApiResponse, ApiResult, ApiResultWithStatus},
    state::AppState,
};

use super::{
    dto::{
        AuthResponse, AuthTokensResponse, EmailStartRequest, EmailVerifyRequest,
        GoogleCallbackQuery, GoogleExchangeRequest, RefreshTokenRequest,
    },
    services,
};

pub async fn email_start(
    State(state): State<AppState>,
    AppJson(body): AppJson<EmailStartRequest>,
) -> ApiResultWithStatus<()> {
    body.validate()?;

    services::email_start(&state, body).await?;

    Ok((
        StatusCode::CREATED,
        Json(ApiResponse::new(None, "OTP sent to your email")),
    ))
}

pub async fn email_verify(
    State(state): State<AppState>,
    AppJson(body): AppJson<EmailVerifyRequest>,
) -> ApiResult<AuthResponse> {
    body.validate()?;

    let auth = services::email_verify(&state, body).await?;

    Ok(Json(ApiResponse::new(Some(auth), "Verified successfully")))
}

pub async fn refresh(
    State(state): State<AppState>,
    AppJson(body): AppJson<RefreshTokenRequest>,
) -> ApiResult<AuthTokensResponse> {
    let tokens = services::refresh(&state, body).await?;

    Ok(Json(ApiResponse::new(Some(tokens), "Token refreshed")))
}

pub async fn logout(
    State(state): State<AppState>,
    AppJson(body): AppJson<RefreshTokenRequest>,
) -> ApiResult<()> {
    services::logout(&state, body).await?;

    Ok(Json(ApiResponse::new(None, "Logged out")))
}

/// The browser lands here from the "Sign in with Google" button and is sent on to Google.
pub async fn google(State(state): State<AppState>) -> Result<Redirect, AppError> {
    let auth_url = services::google_start(&state).await?;

    Ok(Redirect::to(&auth_url))
}

/// Google redirects the browser here. The browser is mid-redirect, so failures
/// also redirect to the frontend (with an error code) instead of returning JSON.
pub async fn google_callback(
    State(state): State<AppState>,
    Query(query): Query<GoogleCallbackQuery>,
) -> Redirect {
    let user_cancelled = query.error.as_deref() == Some("access_denied");

    let param = match services::google_callback(&state, query).await {
        Ok(ticket) => ("ticket", ticket),
        Err(e) => {
            tracing::warn!("Google sign-in failed: {:?}", e);
            let code = if user_cancelled {
                "access_denied"
            } else {
                "google_sign_in_failed"
            };
            ("error", code.to_string())
        }
    };

    Redirect::to(&frontend_callback_url(&state.config.frontend_url, param))
}

/// Next.js calls this with the ticket from the callback URL to get real tokens.
pub async fn google_exchange(
    State(state): State<AppState>,
    AppJson(body): AppJson<GoogleExchangeRequest>,
) -> ApiResult<AuthResponse> {
    body.validate()?;

    let auth = services::google_exchange(&state, body).await?;

    Ok(Json(ApiResponse::new(Some(auth), "Signed in with Google")))
}

/// `FRONTEND_URL/auth/callback?<key>=<value>`, with the value URL-encoded.
fn frontend_callback_url(frontend_url: &str, (key, value): (&str, String)) -> String {
    match Url::parse(frontend_url).and_then(|base| base.join("/auth/callback")) {
        Ok(mut url) => {
            url.query_pairs_mut().append_pair(key, &value);
            url.to_string()
        }
        Err(_) => format!(
            "{}/auth/callback?{}={}",
            frontend_url.trim_end_matches('/'),
            key,
            value
        ),
    }
}
