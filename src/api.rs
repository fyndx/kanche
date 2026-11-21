// SPDX-License-Identifier: MPL-2.0
use crate::identity::{Failure, SignIn, Store};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State, rejection::JsonRejection},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
#[derive(Clone)]
pub struct Code(pub &'static str);
pub(crate) fn error(code: Failure) -> Response {
    let status = match code {
        "invalid_request" | "invalid_redirect_uri" | "invalid_handoff" => StatusCode::BAD_REQUEST,
        "authentication_failed" | "invalid_application_credentials" | "unauthenticated" => {
            StatusCode::UNAUTHORIZED
        }
        "rate_limited" => StatusCode::TOO_MANY_REQUESTS,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    };
    let mut r = status.into_response();
    r.extensions_mut().insert(Code(code));
    r
}
pub(crate) fn key(headers: &HeaderMap) -> &str {
    headers
        .get("authorization")
        .and_then(|x| x.to_str().ok())
        .and_then(|x| x.strip_prefix("Bearer "))
        .unwrap_or("")
}
fn credential(headers: &HeaderMap) -> &str {
    headers
        .get("kanche-session")
        .and_then(|x| x.to_str().ok())
        .filter(|x| x.len() <= 128)
        .unwrap_or("")
}
pub fn routes(store: Store) -> Router {
    let hosted = if store.hosted_enabled() {
        crate::identity::hosted_routes(store.clone())
    } else {
        Router::new()
    };
    Router::new()
        .route("/v1/application", get(application))
        .route("/v1/auth/sign-ins", post(sign_in))
        .route("/v1/sessions/current", get(inspect))
        .route("/v1/sessions/current/renew", post(renew))
        .route("/v1/sessions/current/revoke", post(revoke))
        .layer(DefaultBodyLimit::max(32768))
        .with_state(store)
        .merge(hosted)
}
async fn application(State(s): State<Store>, h: HeaderMap) -> Response {
    match s.application(key(&h)).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => error(e),
    }
}
async fn sign_in(
    State(s): State<Store>,
    h: HeaderMap,
    input: Result<Json<SignIn>, JsonRejection>,
) -> Response {
    let input = match input {
        Ok(Json(v)) => v,
        Err(e) => return e.into_response(),
    };
    match s.sign_in(key(&h), input).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => error(e),
    }
}
async fn session(s: Store, h: HeaderMap, op: &str) -> Response {
    match s.session(key(&h), credential(&h), op).await {
        Ok(_) if op == "revoke" => StatusCode::NO_CONTENT.into_response(),
        Ok(v) => Json(v).into_response(),
        Err(e) => error(e),
    }
}
async fn inspect(State(s): State<Store>, h: HeaderMap) -> Response {
    session(s, h, "inspect").await
}
async fn renew(State(s): State<Store>, h: HeaderMap) -> Response {
    session(s, h, "renew").await
}
async fn revoke(State(s): State<Store>, h: HeaderMap) -> Response {
    session(s, h, "revoke").await
}
