use crate::config::Config;
use anyhow::Context;
use axum::error_handling::HandleErrorLayer;
use axum::http::Uri;
use axum::response::IntoResponse;
use axum::BoxError;
use axum::{body::Body, http::Request, Router};
use sqlx::PgPool;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tower::buffer::BufferLayer;
use tower::ServiceBuilder;
use tower_governor::{errors::display_error, governor::GovernorConfigBuilder, GovernorLayer};
use tower_request_id::{RequestId, RequestIdLayer};
use tracing::info_span;

// Utility modules
use tower::limit::RateLimitLayer;

/// Defines a common error type to use for all request handlers
mod error;

/// Contains all the routes of the application
mod routes;

pub use error::{Error, Result, ResultExt};

use tower_http::trace::TraceLayer;

#[derive(Clone)]
#[allow(dead_code)]
pub struct ApiContext {
    pub config: Arc<Config>,
    pub db: PgPool,
}

pub async fn serve(config: Config, db: PgPool) -> anyhow::Result<()> {
    let governor_conf = Box::new(
        GovernorConfigBuilder::default()
            .per_second(2)
            .burst_size(5)
            .finish()
            .unwrap(),
    );

    let app: Router = Router::<ApiContext>::new()
        .merge(routes::router())
        .layer(
            ServiceBuilder::new()
                .layer(RequestIdLayer)
                .layer(
                    TraceLayer::new_for_http().make_span_with(move |request: &Request<Body>| {
                        let request_id = request
                            .extensions()
                            .get::<RequestId>()
                            .map(ToString::to_string)
                            .unwrap_or_else(|| "unknown".into());

                        info_span!(
                            "request",
                            id = %request_id,
                            method = %request.method(),
                            uri = %request.uri()
                        )
                    }),
                )
                .layer(HandleErrorLayer::new(|e: BoxError| async move {
                    display_error(e)
                }))
                .layer(GovernorLayer {
                    config: Box::leak(governor_conf),
                }),
        )
        .fallback(not_found_handler)
        .with_state(ApiContext {
            config: Arc::new(config),
            db,
        });

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .context("error running HTTP server")
}

async fn not_found_handler(_: Uri) -> impl IntoResponse {
    Error::NotFound
}
