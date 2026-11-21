// SPDX-License-Identifier: MPL-2.0

//! Kanche app-owned authentication and operational foundation.

mod api;
pub mod config;
pub mod identity;

use axum::{
    Router,
    extract::{MatchedPath, Request},
    http::HeaderValue,
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use std::time::Instant;
use tracing::Instrument;
use uuid::Uuid;

/// Construct the application without opening a socket or reading environment.
pub fn app() -> Router {
    app_with_store(None)
}

pub fn app_with_store(store: Option<identity::Store>) -> Router {
    let routes = store.map(api::routes).unwrap_or_default();
    Router::new()
        .merge(routes)
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
        let mut response =
            tokio::time::timeout(std::time::Duration::from_secs(8), next.run(request))
                .await
                .unwrap_or_else(|_| axum::http::StatusCode::SERVICE_UNAVAILABLE.into_response());
        if response.status().is_client_error() || response.status().is_server_error() {
            response = problem(response, &request_id);
        }
        response
            .headers_mut()
            .insert("cache-control", HeaderValue::from_static("no-store"));
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

fn problem(response: Response, request_id: &str) -> Response {
    let original_status = response.status();
    let code = response
        .extensions()
        .get::<api::Code>()
        .map(|x| x.0)
        .unwrap_or(match original_status.as_u16() {
            400 | 422 => "invalid_request",
            413 => "payload_too_large",
            415 => "unsupported_media_type",
            404 => "not_found",
            405 => "method_not_allowed",
            _ => "temporarily_unavailable",
        });
    let status = if original_status.as_u16() == 422 {
        axum::http::StatusCode::BAD_REQUEST
    } else {
        original_status
    };
    let body = serde_json::json!({
        "type": "about:blank", "title": status.canonical_reason().unwrap_or("Error"),
        "status": status.as_u16(), "code": code, "request_id": request_id
    });
    let mut result = (status, axum::Json(body)).into_response();
    result.headers_mut().insert(
        "content-type",
        HeaderValue::from_static("application/problem+json"),
    );
    if status.as_u16() == 429 {
        result
            .headers_mut()
            .insert("retry-after", HeaderValue::from_static("900"));
    }
    result
}
