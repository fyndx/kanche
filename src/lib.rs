// SPDX-License-Identifier: MPL-2.0

//! Health-only service foundation. Authentication contracts are defined separately.

pub mod config;

use axum::{
    Router,
    extract::{MatchedPath, Request},
    http::HeaderValue,
    middleware::{self, Next},
    response::Response,
    routing::get,
};
use std::time::Instant;
use tracing::Instrument;
use uuid::Uuid;

/// Construct the application without opening a socket or reading environment.
pub fn app() -> Router {
    Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .layer(middleware::from_fn(observe))
}

async fn observe(request: Request, next: Next) -> Response {
    // Generate our own ID: client headers must not control log correlation.
    let request_id = Uuid::new_v4().to_string();
    // A route template is safe to log; raw URLs may contain identifiers or tokens.
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map_or("unmatched", MatchedPath::as_str);
    let span = tracing::info_span!("http_request", %request_id, route);
    async move {
        let started = Instant::now();
        let mut response = next.run(request).await;
        response.headers_mut().insert(
            "x-request-id",
            HeaderValue::from_str(&request_id).expect("UUID is a valid header value"),
        );
        tracing::info!(
            status = response.status().as_u16(),
            elapsed_ms = started.elapsed().as_millis() as u64,
            "request completed"
        );
        response
    }
    .instrument(span)
    .await
}
