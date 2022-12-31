mod health_check;

use axum::Router;

use super::ApiContext;

pub fn router() -> Router<ApiContext> {
    Router::new().merge(health_check::router())
}
