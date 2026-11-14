use chrono::Duration;
use hmac::{Hmac, KeyInit, Mac};
use openidconnect::{
    AuthorizationCode, CsrfToken, Nonce, PkceCodeChallenge, PkceCodeVerifier, Scope, TokenResponse,
    core::CoreAuthenticationFlow,
};
use rand::RngExt;
use sha2::Sha256;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

use super::{
    dto::{
        AuthResponse, AuthTokensResponse, EmailStartRequest, EmailVerifyRequest,
        GoogleCallbackQuery, GoogleExchangeRequest, RefreshTokenRequest,
    },
    repo,
};
use crate::handlers::users::repo::{self as users_repo, UserRow};
use crate::{claims::Claims, error::AppError, state::AppState};

pub async fn email_start(state: &AppState, req: EmailStartRequest) -> Result<(), AppError> {
    let limit_count = repo::count_recent_codes(&state.db, &req.email).await?;

    if limit_count.last_minute > 0 {
        return Err(AppError::TooManyRequests(
            "Please wait a minute before requesting another code".to_string(),
        ));
    }
    if limit_count.last_hour >= 5 {
        return Err(AppError::TooManyRequests(
            "Too many codes requested. Try again later".to_string(),
        ));
    }

    let code = format!("{:06}", rand::rng().random_range(0..1_000_000));

    let code_hash = hash_otp_code(&state.config.hmac_secret, &code);

    let mut tx = state.db.begin().await?;
    repo::consume_unused_codes(&mut *tx, &req.email).await?;
    repo::insert_code(&mut *tx, &req.email, &code_hash).await?;

    tx.commit().await?;

    tracing::debug!("login codes for {}: {}", req.email, code);

    // The code is saved first, so a failed send only costs the user the one-minute wait.
    if let Err(e) = state.mailer.send_login_code(&req.email, &code).await {
        tracing::error!("failed to send login code to {}: {}", req.email, e);
        return Err(AppError::ServiceUnavailable(
            "We couldn't send your code. Please try again in a minute".to_string(),
        ));
    }

    Ok(())
}

pub async fn email_verify(
    state: &AppState,
    req: EmailVerifyRequest,
) -> Result<AuthResponse, AppError> {
    let active_code = repo::find_active_code(&state.db, &req.email)
        .await?
        .ok_or_else(|| AppError::BadRequest("Invalid or expired code".to_string()))?;

    // Count the attempt first, so a burst of parallel guesses gets at most 5 checks.
    if !repo::use_attempt(&state.db, active_code.id).await? {
        return Err(AppError::BadRequest("Invalid or expired code".to_string()));
    }

    if !verify_otp_code(&state.config.hmac_secret, &req.code, &active_code.code_hash)? {
        return Err(AppError::BadRequest("Invalid or expired code".to_string()));
    }

    let mut tx = state.db.begin().await?;

    let is_code_valid = repo::consume_code(&mut *tx, active_code.id).await?;

    if !is_code_valid {
        return Err(AppError::BadRequest("Invalid or expired code".to_string()));
    }

    let user = repo::find_or_create_user_by_email(&mut *tx, &req.email).await?;

    let (tokens, jti, exp) = generate_auth_token(user.id, user.email.clone(), state)?;

    // Same transaction: the code is only used up if the refresh token is saved too.
    repo::store_refresh_token(&mut *tx, user.id, jti, exp).await?;

    tx.commit().await?;

    Ok(AuthResponse {
        user: user.into(),
        tokens,
    })
}

/// Rotates the refresh token: the old one is revoked and a new pair is issued.
/// If the old one was already used, someone may have stolen it, so every
/// session for that user is revoked.
pub async fn refresh(
    state: &AppState,
    req: RefreshTokenRequest,
) -> Result<AuthTokensResponse, AppError> {
    let claims = Claims::decode_jwt(&req.refresh_token, &state.jwt_refresh_decoded_secret)
        .map_err(|_| AppError::Unauthorized("Invalid token".to_string()))?
        .claims;

    let mut tx = state.db.begin().await?;

    let old_jti = repo::revoke_refresh_token_if_valid(&mut *tx, claims.sub, claims.jti).await?;

    if old_jti.is_none() {
        repo::revoke_all_refresh_tokens(&mut *tx, claims.sub).await?;
        tx.commit().await?;
        return Err(AppError::Unauthorized("Unauthorized".to_string()));
    }

    let (tokens, jti, exp) = generate_auth_token(claims.sub, claims.email, state)?;

    repo::store_refresh_token(&mut *tx, claims.sub, jti, exp).await?;
    tx.commit().await?;

    Ok(tokens)
}

/// Ends the session that owns this refresh token. The access token stays valid
/// until it expires, so the client should drop both.
pub async fn logout(state: &AppState, req: RefreshTokenRequest) -> Result<(), AppError> {
    let claims = Claims::decode_jwt(&req.refresh_token, &state.jwt_refresh_decoded_secret)
        .map_err(|_| AppError::Unauthorized("Invalid token".to_string()))?
        .claims;

    repo::revoke_refresh_token(&state.db, claims.sub, claims.jti).await?;

    Ok(())
}

/// Starts "Sign in with Google": saves fresh state, PKCE verifier and nonce,
/// and returns the Google URL to send the browser to.
pub async fn google_start(state: &AppState) -> Result<String, AppError> {
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    // `openid` is always requested; email + profile give us the address, name and avatar.
    let (auth_url, csrf_state, nonce) = state
        .google
        .client
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .add_scope(Scope::new("email".to_string()))
        .add_scope(Scope::new("profile".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    repo::insert_oauth_state(
        &state.db,
        csrf_state.secret(),
        pkce_verifier.secret(),
        nonce.secret(),
    )
    .await?;

    Ok(auth_url.to_string())
}

/// Finishes "Sign in with Google" and returns a one-time ticket for the frontend.
pub async fn google_callback(
    state: &AppState,
    query: GoogleCallbackQuery,
) -> Result<String, AppError> {
    // 1. Google sends `error` when the user cancels or something fails on its side.
    if let Some(error) = query.error {
        return Err(AppError::Unauthorized(format!(
            "Google returned an error: {}",
            error
        )));
    }

    let (Some(code), Some(csrf_state)) = (query.code, query.state) else {
        return Err(AppError::BadRequest("Missing code or state".to_string()));
    };

    // 2. The state must be one we issued, unused and unexpired. Deleting it here
    //    means a replayed callback URL fails.
    let saved = repo::take_oauth_state(&state.db, &csrf_state)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Invalid or expired sign-in attempt".to_string()))?;

    // 3. Swap the code for tokens. The PKCE verifier proves we started this flow.
    let token_response = state
        .google
        .client
        .exchange_code(AuthorizationCode::new(code))
        .map_err(|e| AppError::Internal(format!("Google client is missing a token URL: {}", e)))?
        .set_pkce_verifier(PkceCodeVerifier::new(saved.pkce_verifier))
        .request_async(&state.google.http)
        .await
        .map_err(|e| AppError::Unauthorized(format!("Google code exchange failed: {}", e)))?;

    // 4. Verify the ID token: signature, issuer, audience, expiry and our nonce.
    let id_token = token_response
        .id_token()
        .ok_or_else(|| AppError::Unauthorized("Google did not return an ID token".to_string()))?;

    let verifier = state
        .google
        .id_token_verifier()
        .await
        .map_err(AppError::Internal)?;

    let claims = id_token
        .claims(&verifier, &Nonce::new(saved.nonce))
        .map_err(|e| AppError::Unauthorized(format!("Invalid Google ID token: {}", e)))?;

    let google_sub = claims.subject().as_str();

    let email = claims
        .email()
        .map(|e| e.as_str())
        .ok_or_else(|| AppError::Unauthorized("Google account has no email".to_string()))?;

    // Linking by email is only safe if Google has confirmed the address.
    if claims.email_verified() != Some(true) {
        return Err(AppError::Unauthorized(
            "Google email is not verified".to_string(),
        ));
    }

    let full_name = claims.name().and_then(|n| n.get(None)).map(|n| n.as_str());
    let avatar_url = claims
        .picture()
        .and_then(|p| p.get(None))
        .map(|p| p.as_str());

    // 5. Find, link or create the user, and issue the ticket in one transaction.
    let mut tx = state.db.begin().await?;

    let user =
        find_link_or_create_google_user(&mut tx, google_sub, email, full_name, avatar_url).await?;

    // 6. A one-time ticket, not tokens, goes in the redirect URL.
    let ticket = hex::encode(rand::rng().random::<[u8; 32]>());
    let ticket_hash = hash_secret(&state.config.hmac_secret, &ticket);

    repo::insert_auth_ticket(&mut *tx, &ticket_hash, user.id).await?;

    tx.commit().await?;

    Ok(ticket)
}

/// Swaps the one-time ticket from the Google callback for the same response
/// email sign-in returns.
pub async fn google_exchange(
    state: &AppState,
    req: GoogleExchangeRequest,
) -> Result<AuthResponse, AppError> {
    let ticket_hash = hash_secret(&state.config.hmac_secret, &req.ticket);

    let mut tx = state.db.begin().await?;

    let user_id = repo::consume_auth_ticket(&mut *tx, &ticket_hash)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Invalid or expired ticket".to_string()))?;

    let user = users_repo::find_by_id(&mut *tx, user_id)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Invalid or expired ticket".to_string()))?;

    let (tokens, jti, exp) = generate_auth_token(user.id, user.email.clone(), state)?;

    // Same transaction: the ticket is only used up if the refresh token is saved too.
    repo::store_refresh_token(&mut *tx, user.id, jti, exp).await?;

    tx.commit().await?;

    Ok(AuthResponse {
        user: user.into(),
        tokens,
    })
}

/// Match on Google's stable `sub` first, then link an existing email account,
/// otherwise create a new user.
async fn find_link_or_create_google_user(
    tx: &mut sqlx::PgConnection,
    google_sub: &str,
    email: &str,
    full_name: Option<&str>,
    avatar_url: Option<&str>,
) -> Result<UserRow, AppError> {
    if let Some(user) = repo::find_user_by_google_sub(&mut *tx, google_sub).await? {
        return Ok(user);
    }

    if let Some(user) =
        repo::link_google_to_email_user(&mut *tx, email, google_sub, full_name, avatar_url).await?
    {
        return Ok(user);
    }

    Ok(repo::create_google_user(&mut *tx, email, google_sub, full_name, avatar_url).await?)
}

/// Hex-encoded HMAC of the code. This is what gets stored, never the code itself.
fn hash_otp_code(hmac_secret: &str, code: &str) -> String {
    hash_secret(hmac_secret, code)
}

/// Checks a submitted code against the stored hash in constant time.
fn verify_otp_code(hmac_secret: &str, code: &str, code_hash: &str) -> Result<bool, AppError> {
    let expected = hex::decode(code_hash)?;
    Ok(keyed_mac(hmac_secret, code).verify_slice(&expected).is_ok())
}

/// Hex-encoded HMAC of a secret value (login code or ticket), for storing.
fn hash_secret(hmac_secret: &str, value: &str) -> String {
    hex::encode(keyed_mac(hmac_secret, value).finalize().into_bytes())
}

fn keyed_mac(hmac_secret: &str, value: &str) -> HmacSha256 {
    let mut mac = HmacSha256::new_from_slice(hmac_secret.as_bytes())
        .expect("HMAC can take a key of any size");
    mac.update(value.as_bytes());
    mac
}

fn generate_auth_token(
    sub: Uuid,
    email: String,
    state: &AppState,
) -> Result<(AuthTokensResponse, Uuid, usize), AppError> {
    let access_token_claims = Claims::new(
        sub,
        email.clone(),
        Duration::minutes(state.config.access_token_ttl_minutes),
    );

    let refresh_token_claims = Claims::new(
        sub,
        email,
        Duration::days(state.config.refresh_token_ttl_days),
    );

    let token = AuthTokensResponse::new(
        &access_token_claims,
        &refresh_token_claims,
        &state.jwt_encoded_secret,
        &state.jwt_refresh_encoded_secret,
    )?;

    Ok((token, refresh_token_claims.jti, refresh_token_claims.exp))
}
