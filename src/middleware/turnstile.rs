use axum::{
    extract::ConnectInfo,
    http::{HeaderMap, Method, StatusCode},
    middleware::Next,
    response::Response,
};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::{
    net::SocketAddr,
    sync::OnceLock,
    time::{Duration, Instant},
};
use tracing::{warn, error};

// Global token cache to allow reuse of validated tokens
// Using OnceLock for thread-safe lazy initialization
static TOKEN_CACHE: OnceLock<DashMap<String, Instant>> = OnceLock::new();

// Token validity duration - tokens can be reused for 5 minutes
const TOKEN_CACHE_DURATION: Duration = Duration::from_secs(300);

fn get_token_cache() -> &'static DashMap<String, Instant> {
    TOKEN_CACHE.get_or_init(|| DashMap::new())
}

#[derive(Debug, Serialize, Deserialize)]
struct TurnstileVerifyRequest {
    secret: String,
    response: String,
    remoteip: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TurnstileVerifyResponse {
    success: bool,
    #[serde(rename = "error-codes")]
    error_codes: Option<Vec<String>>,
    challenge_ts: Option<String>,
    hostname: Option<String>,
}

const TURNSTILE_VERIFY_URL: &str = "https://challenges.cloudflare.com/turnstile/v0/siteverify";

pub async fn turnstile_verification_middleware(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    method: Method,
    request: axum::http::Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // Only verify POST requests
    if method != Method::POST {
        return Ok(next.run(request).await);
    }

    // Exclude certain endpoints from Turnstile verification
    let uri = request.uri();
    let path = uri.path();
    
    // Skip Turnstile verification for stats and health endpoints
    if path.starts_with("/api/stats") || path == "/api/health" {
        return Ok(next.run(request).await);
    }

    // Skip Turnstile verification in development mode
    if std::env::var("TURNSTILE_BYPASS").unwrap_or_default() == "true" {
        tracing::info!("Turnstile verification bypassed for development");
        return Ok(next.run(request).await);
    }

    // Get secret key from environment
    let secret_key = std::env::var("TURNSTILE_SECRET_KEY")
        .unwrap_or_else(|_| {
            error!("TURNSTILE_SECRET_KEY environment variable not set");
            String::new()
        });

    if secret_key.is_empty() {
        error!("Turnstile secret key is empty - consider setting TURNSTILE_BYPASS=true for development");
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Extract Turnstile token from headers
    let turnstile_token = match headers.get("CF-Turnstile-Token") {
        Some(token_header) => match token_header.to_str() {
            Ok(token) => token,
            Err(_) => {
                warn!("Invalid Turnstile token header format");
                return Err(StatusCode::BAD_REQUEST);
            }
        },
        None => {
            warn!("Missing Turnstile token in POST request");
            return Err(StatusCode::FORBIDDEN);
        }
    };

    // Get client IP for verification
    let client_ip = extract_client_ip(&headers, addr);

    // Check if token is cached and still valid
    let now = Instant::now();
    let token_cache = get_token_cache();
    if let Some(cached_time) = token_cache.get(turnstile_token) {
        if now.duration_since(*cached_time) < TOKEN_CACHE_DURATION {
            return Ok(next.run(request).await);
        } else {
            // Token expired, remove from cache
            token_cache.remove(turnstile_token);
        }
    }

    // Verify token with Cloudflare
    match verify_turnstile_token(turnstile_token, &client_ip, &secret_key).await {
        Ok(true) => {
            // Cache the successful token
            token_cache.insert(turnstile_token.to_string(), now);
            Ok(next.run(request).await)
        }
        Ok(false) => {
            warn!("Turnstile verification failed for IP: {}", client_ip);
            Err(StatusCode::FORBIDDEN)
        }
        Err(e) => {
            error!("Turnstile verification error: {}", e);
            Err(StatusCode::SERVICE_UNAVAILABLE)
        }
    }
}

async fn verify_turnstile_token(token: &str, client_ip: &str, secret_key: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::new();
    
    let verify_request = TurnstileVerifyRequest {
        secret: secret_key.to_string(),
        response: token.to_string(),
        remoteip: Some(client_ip.to_string()),
    };

    let response = client
        .post(TURNSTILE_VERIFY_URL)
        .form(&verify_request)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("Turnstile API returned status: {}", response.status()).into());
    }

    let verify_response: TurnstileVerifyResponse = response.json().await?;

    if !verify_response.success {
        if let Some(error_codes) = &verify_response.error_codes {
            warn!("Turnstile verification failed with errors: {:?}", error_codes);
        }
        return Ok(false);
    }

    Ok(true)
}

fn extract_client_ip(headers: &HeaderMap, addr: SocketAddr) -> String {
    // Check for common proxy headers
    if let Some(forwarded_for) = headers.get("X-Forwarded-For") {
        if let Ok(forwarded_str) = forwarded_for.to_str() {
            // Take the first IP in the chain
            if let Some(first_ip) = forwarded_str.split(',').next() {
                return first_ip.trim().to_string();
            }
        }
    }

    if let Some(real_ip) = headers.get("X-Real-IP") {
        if let Ok(real_ip_str) = real_ip.to_str() {
            return real_ip_str.to_string();
        }
    }

    if let Some(forwarded) = headers.get("Forwarded") {
        if let Ok(forwarded_str) = forwarded.to_str() {
            // Parse RFC 7239 Forwarded header
            for pair in forwarded_str.split(';') {
                if let Some((key, value)) = pair.split_once('=') {
                    if key.trim().eq_ignore_ascii_case("for") {
                        return value.trim().trim_matches('"').to_string();
                    }
                }
            }
        }
    }

    // Fall back to direct connection IP
    addr.ip().to_string()
}

// Cleanup function to remove expired tokens from cache
// This should be called periodically to prevent memory leaks
#[allow(dead_code)]
pub fn cleanup_expired_tokens() {
    let now = Instant::now();
    let token_cache = get_token_cache();
    token_cache.retain(|_, cached_time| {
        now.duration_since(*cached_time) < TOKEN_CACHE_DURATION
    });
}
