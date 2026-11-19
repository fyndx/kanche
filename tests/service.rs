// SPDX-License-Identifier: MPL-2.0

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use kanche::{app, config::Config};
use tower::ServiceExt;
use uuid::Uuid;

#[test]
fn configuration_is_explicit_and_errors_do_not_echo_input() {
    let defaults = Config::parse(None, None).unwrap();
    assert!(defaults.bind_addr.ip().is_loopback());
    assert_eq!(defaults.bind_addr.port(), 8080);
    assert_eq!(
        Config::parse(Some("[::1]:0"), Some("warn"))
            .unwrap()
            .bind_addr
            .port(),
        0
    );
    for bad in ["", "localhost:8080", "127.0.0.1:65536", "sensitive-input"] {
        let error = Config::parse(Some(bad), None).unwrap_err().to_string();
        assert!(error.contains("KANCHE_BIND_ADDR"));
        assert!(!error.contains("sensitive-input"));
    }
    for bad in ["", "off", "INFO", "sensitive-input"] {
        let error = Config::parse(None, Some(bad)).unwrap_err().to_string();
        assert!(error.contains("KANCHE_LOG_LEVEL"));
        assert!(!error.contains("sensitive-input"));
    }
}

#[tokio::test]
async fn health_and_fallback_get_fresh_server_generated_request_ids() {
    let mut ids = Vec::new();
    for (method, path, status) in [
        ("GET", "/healthz", StatusCode::OK),
        ("GET", "/unknown?token=sensitive", StatusCode::NOT_FOUND),
        ("POST", "/healthz", StatusCode::METHOD_NOT_ALLOWED),
        ("GET", "/healthz", StatusCode::OK),
    ] {
        let response = app()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(path)
                    .header("x-request-id", "client-controlled")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), status);
        let id = response.headers()["x-request-id"]
            .to_str()
            .unwrap()
            .to_owned();
        assert!(Uuid::parse_str(&id).is_ok());
        assert!(!ids.contains(&id));
        ids.push(id);
        if status == StatusCode::OK {
            assert_eq!(to_bytes(response.into_body(), 1024).await.unwrap(), "ok");
        }
    }
}
