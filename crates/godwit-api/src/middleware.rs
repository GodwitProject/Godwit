use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::Response,
};
use godwit_auth::{api_keys::verify_key, jwt::verify};

use crate::state::AppState;

pub fn extract_token(header: &str) -> Option<&str> {
    header.strip_prefix("Bearer ")
}

pub async fn api_key_auth(
    State(state): State<Arc<AppState>>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Skip API key auth for admin routes — they use JWT instead
    if req.uri().path().starts_with("/api/v1") {
        return Ok(next.run(req).await);
    }

    let auth = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(extract_token)
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Fast path: cache lookup by raw key.
    if let Some(key) = state.api_key_cache.get(&auth.to_string()).await {
        if !key.disabled
            && key
                .expires_at
                .map(|e| e > chrono::Utc::now())
                .unwrap_or(true)
        {
            req.extensions_mut().insert(key);
            return Ok(next.run(req).await);
        }
    }

    // Fallback: database lookup by prefix.
    let prefix = godwit_auth::api_keys::extract_prefix(auth);
    let candidates = state
        .api_key_repo
        .get_by_prefix(&prefix)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let key = candidates
        .into_iter()
        .find(|k| verify_key(auth, &k.key_hash))
        .ok_or(StatusCode::UNAUTHORIZED)?;
    if key.disabled
        || key
            .expires_at
            .map(|e| e < chrono::Utc::now())
            .unwrap_or(false)
    {
        return Err(StatusCode::UNAUTHORIZED);
    }
    state
        .api_key_cache
        .insert(auth.to_string(), key.clone())
        .await;
    req.extensions_mut().insert(key);
    Ok(next.run(req).await)
}

pub async fn jwt_auth(
    State(state): State<Arc<AppState>>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(extract_token)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let claims =
        verify(&state.config.auth.jwt_secret, auth).map_err(|_| StatusCode::UNAUTHORIZED)?;
    req.extensions_mut().insert(claims);
    Ok(next.run(req).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_bearer_token() {
        assert_eq!(
            extract_token("Bearer sk-godwit-abc123"),
            Some("sk-godwit-abc123")
        );
        assert_eq!(extract_token("Basic abc"), None);
    }
}
