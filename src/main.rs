use std::{net::SocketAddr, time::Duration};

use axum::{
    Router,
    http::{
        HeaderValue, Method,
        header::{AUTHORIZATION, CONTENT_TYPE},
    },
};
use dotenvy::dotenv;
use sqlx::postgres::PgPoolOptions;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::EnvFilter;

use crate::{
    config::Config,
    google::GoogleOidc,
    handlers::{app_routes, auth::jobs::spawn_cleanup_job, fallback::not_found},
    state::AppState,
};

mod claims;
mod config;
mod error;
mod extractors;
mod feature_flags;
mod google;
mod handlers;
mod indexer;
mod response;
mod state;
mod utils;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let tracing_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("wage3_backend=debug,tower_http=info,sqlx=warn"));

    tracing_subscriber::fmt()
        .with_env_filter(tracing_filter)
        .init();

    let config = Config::new().expect("Failed to load config");

    let port = config.port;

    let pool = PgPoolOptions::new()
        .acquire_timeout(Duration::new(5, 0))
        .connect(&config.database_url)
        .await
        .expect("Failed to connect to db");

    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("Failed to migrate");

    spawn_cleanup_job(pool.clone());

    indexer::spawn_indexer(
        pool.clone(),
        indexer::IndexerConfig {
            rpc_url: config.rpc_url.clone(),
            chain_id: config.chain_id,
            contract: config.batcher_address,
            start_block: config.batcher_start_block,
            max_range: config.indexer_max_range,
        },
    )
    .expect("Failed to start indexer");

    let google = GoogleOidc::discover(&config)
        .await
        .expect("Failed to set up Google sign-in");

    // Browsers send Origin without a trailing slash, so FRONTEND_URL must match that.
    let frontend_origin = config
        .frontend_url
        .trim_end_matches('/')
        .parse::<HeaderValue>()
        .expect("FRONTEND_URL is not a valid origin");

    // Auth is a Bearer header, not cookies, so credentials stay off.
    let cors = CorsLayer::new()
        .allow_origin(frontend_origin)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
        ])
        .allow_headers([AUTHORIZATION, CONTENT_TYPE])
        .max_age(Duration::from_secs(60 * 60));

    let state = AppState::new(config, pool, google);

    let tracing_layer = ServiceBuilder::new().layer(TraceLayer::new_for_http());

    let app = Router::new()
        .nest("/api", app_routes())
        .layer(tracing_layer)
        .with_state(state)
        .fallback(not_found)
        .layer(cors);

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .expect("Failed to connect to port");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .expect("Failed to start server");
}
