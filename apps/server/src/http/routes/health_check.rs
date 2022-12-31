use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;

use crate::http::ApiContext;

pub fn router() -> Router<ApiContext> {
    Router::new().route("/", get(health))
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}
