use axum::{
    http::{StatusCode, HeaderMap},
    response::Response,
    Router,
    routing::get,
};
use std::sync::Arc;
use crate::metrics::get_metrics;
use crate::state::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/metrics", get(metrics_handler))
}

async fn metrics_handler() -> Result<Response<String>, StatusCode> {
    match get_metrics() {
        Ok(metrics_text) => {
            let mut headers = HeaderMap::new();
            headers.insert(
                "content-type",
                "text/plain; version=0.0.4".parse().unwrap(),
            );
            
            let mut response = Response::new(metrics_text);
            *response.headers_mut() = headers;
            Ok(response)
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
