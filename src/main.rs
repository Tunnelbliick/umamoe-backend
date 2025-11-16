use axum::{
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use sqlx::PgPool;
use std::net::SocketAddr;
use tower::ServiceBuilder;
use tower_http::{
    cors::{CorsLayer, Any},
    trace::TraceLayer,
};
use tracing::{info, warn, Level};
use tracing_subscriber::EnvFilter;

mod models;
mod handlers;
mod database;
mod errors;
mod middleware;

use handlers::{circles, search, stats, tasks, sharing};

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing with reduced SQL verbosity
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_env_filter(
            EnvFilter::new("honsemoe_backend=info,sqlx=warn,info")
        )
        .init();

    // Load environment variables
    dotenvy::dotenv().ok();

    // Database connection
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    
    let pool = database::create_pool(&database_url)
        .await
        .expect("Failed to connect to PostgreSQL");

    // Run migrations with better error handling (can be disabled via env var)
    let skip_migrations = std::env::var("SKIP_MIGRATIONS")
        .map(|v| v.to_lowercase() == "true" || v == "1")
        .unwrap_or(false);
    
    if skip_migrations {
        warn!("⚠️ Skipping migrations due to SKIP_MIGRATIONS=true");
    } else {
        match sqlx::migrate!("./migrations").run(&pool).await {
            Ok(_) => info!("✅ Migrations completed successfully"),
            Err(sqlx::migrate::MigrateError::VersionMismatch(version)) => {
                warn!("⚠️  Migration version mismatch: {}", version);
                warn!("Database has different migration state than expected");
                warn!("Consider resetting migrations: DROP TABLE _sqlx_migrations;");
            }
            Err(e) => {
                warn!("❌ Failed to run migrations: {}", e);
                warn!("Continuing without migrations (set SKIP_MIGRATIONS=true to suppress this warning)");
            }
        }
    }

    let state = AppState { db: pool };

    // Configure CORS - more permissive for development, strict for production
    let is_development = std::env::var("DEBUG_MODE").unwrap_or_default() == "true";
    
    let cors = if is_development {
        info!("🔓 Development mode: Using permissive CORS");
        CorsLayer::new()
            .allow_origin(Any)
            .allow_credentials(false) // Can't use credentials with allow_origin(Any)
    } else {
        let allowed_origins = std::env::var("ALLOWED_ORIGINS")
            .unwrap_or_else(|_| "https://honse.moe,https://www.honse.moe,https://uma.moe,https://www.uma.moe,http://honse.moe,http://www.honse.moe,http://uma.moe,http://www.uma.moe".to_string());
        
        info!("🔍 Raw ALLOWED_ORIGINS: {}", allowed_origins);
        
        let origins: Result<Vec<_>, _> = allowed_origins
            .split(',')
            .map(|origin| {
                let trimmed = origin.trim();
                info!("  📍 Parsing origin: '{}'", trimmed);
                trimmed.parse()
            })
            .collect();
        
        match origins {
            Ok(parsed_origins) => {
                info!("🔒 Production mode: CORS configured for origins: {}", allowed_origins);
                for origin in &parsed_origins {
                    info!("  - Allowed origin: {:?}", origin);
                }
                CorsLayer::new()
                    .allow_origin(parsed_origins)
                    .allow_credentials(true)
            },
            Err(e) => {
                warn!("⚠️ Failed to parse ALLOWED_ORIGINS, using defaults: {}", e);
                let default_origins = vec![
                    "https://honse.moe".parse().unwrap(),
                    "https://www.honse.moe".parse().unwrap(),
                    "https://uma.moe".parse().unwrap(),
                    "https://www.uma.moe".parse().unwrap(),
                    "http://honse.moe".parse().unwrap(),
                    "http://www.honse.moe".parse().unwrap(),
                    "http://uma.moe".parse().unwrap(),
                    "http://www.uma.moe".parse().unwrap(),
                ];
                info!("🔒 Using fallback origins with {} entries", default_origins.len());
                for origin in &default_origins {
                    info!("  - Fallback origin: {:?}", origin);
                }
                CorsLayer::new()
                    .allow_origin(default_origins)
                    .allow_credentials(true)
            }
        }
    }
    .allow_methods([
        axum::http::Method::GET,
        axum::http::Method::POST,
        axum::http::Method::PUT,
        axum::http::Method::DELETE,
        axum::http::Method::OPTIONS,
    ])
    .allow_headers([
        axum::http::header::CONTENT_TYPE,
        axum::http::header::AUTHORIZATION,
        axum::http::header::ACCEPT,
        axum::http::header::USER_AGENT,
        axum::http::header::REFERER,
        axum::http::header::ORIGIN,
        "CF-Turnstile-Token".parse().unwrap(),
    ]);

    // Build the application with proper routing and middleware
    // Public endpoints (no Turnstile, permissive CORS)
    let public_routes = Router::new()
        .nest("/api/circles", circles::router())
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive()) // Allow all origins for public API
        )
        .with_state(state.clone());

    // Protected endpoints (Turnstile + restricted CORS)
    let protected_routes = Router::new()
        .route("/api/health", get(health_check))
        .nest("/api/stats", stats::router())
        .nest("/api/tasks", tasks::router())
        .nest("/api/v3/tasks", tasks::router())
        .nest("/api/v3", search::router())
        .nest("/", sharing::router())
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(axum::middleware::from_fn(middleware::turnstile_verification_middleware))
                .layer(cors)
        )
        .with_state(state);

    // Merge public and protected routes
    let app = public_routes.merge(protected_routes);

    // Server configuration
    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "3001".to_string())
        .parse::<u16>()
        .expect("PORT must be a valid number");

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    
    info!("🚀 Server starting on http://{}:{}", host, port);

    // Start the server - compatible with both Axum 0.6 and 0.7
    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await?;

    Ok(())
}

async fn health_check() -> Result<Json<serde_json::Value>, StatusCode> {
    Ok(Json(serde_json::json!({
        "status": "healthy",
        "service": "honsemoe-backend",
        "timestamp": chrono::Utc::now(),
        "version": "1.0.0",
        "endpoints": {
            "search": "/api/v3/search",
            "stats": "/api/stats", 
            "tasks": "/api/tasks",
            "circles": "/api/circles",
            "health": "/api/health"
        }
    })))
}
