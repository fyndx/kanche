// SPDX-License-Identifier: MPL-2.0
use axum::{
    body::{Body, to_bytes},
    http::{HeaderMap, Request},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use kanche::{
    app_with_store,
    identity::{
        HostedExchange, HostedRegistration, HostedTransaction, Provision, SignIn, Store, digest,
        secret,
    },
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tower::ServiceExt;
use url::Url;
const CALLBACK: &str = "http://localhost:3010/api/kanche/callback";
const ORIGIN: &str = "http://127.0.0.1:18080";
struct HttpResult {
    status: u16,
    headers: HeaderMap,
    body: String,
}
async fn request(
    s: &Store,
    route: (&str, &str),
    key: Option<&str>,
    cookie: Option<&str>,
    origin: Option<&str>,
    content_type: Option<&str>,
    body: String,
) -> HttpResult {
    let (method, path) = route;
    let mut builder = Request::builder().method(method).uri(path);
    if let Some(key) = key {
        builder = builder.header("authorization", format!("Bearer {key}"));
    }
    if let Some(cookie) = cookie {
        builder = builder.header("cookie", cookie);
    }
    if let Some(origin) = origin {
        builder = builder.header("origin", origin);
    }
    if let Some(content_type) = content_type {
        builder = builder.header("content-type", content_type);
    }
    let result = app_with_store(Some(s.clone()))
        .oneshot(builder.body(Body::from(body)).unwrap())
        .await
        .unwrap();
    let status = result.status().as_u16();
    let headers = result.headers().clone();
    assert!(headers.get("x-request-id").is_some());
    assert!(headers["cache-control"] == "no-store");
    let body =
        String::from_utf8(to_bytes(result.into_body(), 32768).await.unwrap().to_vec()).unwrap();
    HttpResult {
        status,
        headers,
        body,
    }
}
fn provision(tenant: &str, app: &str, user: &str, key: &str, password: &str) -> Provision {
    Provision {
        tenant_id: tenant.into(),
        application_id: app.into(),
        user_id: user.into(),
        email: format!("{user}@example.invalid"),
        password: password.into(),
        application_secret: key.into(),
    }
}
fn registration(app: &str) -> HostedRegistration {
    HostedRegistration {
        application_id: app.into(),
        redirect_uri: CALLBACK.into(),
        development: true,
    }
}
fn challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}
fn create_input(state: &str, verifier: &str) -> HostedTransaction {
    HostedTransaction {
        redirect_uri: CALLBACK.into(),
        state: state.into(),
        code_challenge: challenge(verifier),
        code_challenge_method: "S256".into(),
    }
}
fn exchange_input(code: &str, verifier: &str) -> HostedExchange {
    HostedExchange {
        code: code.into(),
        code_verifier: verifier.into(),
        redirect_uri: CALLBACK.into(),
    }
}
fn hidden(body: &str, name: &str) -> String {
    let prefix = format!("name=\"{name}\" value=\"");
    body.split_once(&prefix)
        .expect("expected hidden field")
        .1
        .split('"')
        .next()
        .unwrap()
        .into()
}
fn handle(value: &Value) -> String {
    Url::parse(value["authorization_url"].as_str().unwrap())
        .unwrap()
        .query_pairs()
        .find(|(k, _)| k == "transaction")
        .unwrap()
        .1
        .into_owned()
}
fn form(handle: &str, csrf: &str, email: &str, password: &str) -> String {
    url::form_urlencoded::Serializer::new(String::new())
        .append_pair("transaction", handle)
        .append_pair("csrf_token", csrf)
        .append_pair("email", email)
        .append_pair("password", password)
        .finish()
}
async fn get_form(s: &Store, handle: &str, cookie: Option<&str>) -> (String, String) {
    let response = request(
        s,
        ("GET", &format!("/hosted/login?transaction={handle}")),
        None,
        cookie,
        None,
        None,
        String::new(),
    )
    .await;
    assert!(response.status == 200, "valid hosted form failed");
    assert!(
        response.headers["content-type"]
            .to_str()
            .unwrap()
            .starts_with("text/html")
    );
    assert!(
        response.headers["content-security-policy"]
            .to_str()
            .unwrap()
            .contains("frame-ancestors 'none'")
    );
    assert!(response.headers["referrer-policy"] == "same-origin");
    assert!(
        response.headers["content-security-policy"]
            .to_str()
            .unwrap()
            .contains(&format!("form-action 'self' {CALLBACK};"))
    );
    let cookie = response.headers["set-cookie"].to_str().unwrap();
    assert!(
        cookie.contains("HttpOnly")
            && cookie.contains("SameSite=Lax")
            && !cookie.contains("Domain=")
    );
    assert!(cookie.contains("Max-Age=300"));
    (
        cookie.split(';').next().unwrap().into(),
        hidden(&response.body, "csrf_token"),
    )
}
async fn password_response(
    s: &Store,
    handle: &str,
    cookie: &str,
    csrf: &str,
    email: &str,
    password: &str,
) -> HttpResult {
    request(
        s,
        ("POST", "/hosted/login"),
        None,
        Some(cookie),
        Some(ORIGIN),
        Some("application/x-www-form-urlencoded"),
        form(handle, csrf, email, password),
    )
    .await
}
async fn authenticated_code(
    s: &Store,
    key: &str,
    password: &str,
    verifier: &str,
) -> (String, String, String, String) {
    let state = secret();
    let transaction = s
        .create_hosted_transaction(key, create_input(&state, verifier))
        .await
        .unwrap();
    let handle = handle(&transaction);
    let (cookie, csrf) = get_form(s, &handle, None).await;
    let response = password_response(
        s,
        &handle,
        &cookie,
        &csrf,
        "hosted_user@example.invalid",
        password,
    )
    .await;
    assert!(
        response.status == 303,
        "valid hosted password did not complete"
    );
    assert!(response.body.is_empty());
    let location = Url::parse(response.headers["location"].to_str().unwrap()).unwrap();
    assert!(location.origin().ascii_serialization() == "http://localhost:3010");
    assert!(location.path() == "/api/kanche/callback");
    let params: Vec<_> = location.query_pairs().collect();
    assert!(params.len() == 2);
    let code = params
        .iter()
        .find(|(k, _)| k == "code")
        .unwrap()
        .1
        .to_string();
    assert!(params.iter().find(|(k, _)| k == "state").unwrap().1 == state);
    (code, handle, cookie, csrf)
}
#[tokio::test]
#[ignore = "requires disposable PostgreSQL; run make test-integration"]
async fn hosted_profile_invariants() {
    let base = std::env::var("KANCHE_TEST_DATABASE_URL").expect("disposable database required");
    let initial = Store::new(&base).unwrap();
    let admin = initial.connect().await.unwrap();
    admin
        .batch_execute("CREATE DATABASE hosted_profile_evidence")
        .await
        .unwrap();
    let mut url = Url::parse(&base).unwrap();
    url.set_path("/hosted_profile_evidence");
    let plain = Store::new(url.as_str()).unwrap();
    let db = plain.connect().await.unwrap();
    // Construct the actual prior schema, provision/login, then migrate forward.
    db.batch_execute(include_str!("../migrations/0001_app_login.sql"))
        .await
        .unwrap();
    db.batch_execute(
        "CREATE TABLE kanche_migrations(version integer PRIMARY KEY,digest text NOT NULL)",
    )
    .await
    .unwrap();
    db.execute(
        "INSERT INTO kanche_migrations VALUES(1,$1)",
        &[&digest(include_str!("../migrations/0001_app_login.sql"))],
    )
    .await
    .unwrap();
    let key = secret();
    let password = secret();
    let key2 = secret();
    let foreign = secret();
    plain
        .provision(provision(
            "hosted_tenant",
            "hosted_app",
            "hosted_user",
            &key,
            &password,
        ))
        .await
        .unwrap();
    let before = plain
        .sign_in(
            &key,
            SignIn {
                email: "hosted_user@example.invalid".into(),
                password: password.clone(),
            },
        )
        .await
        .unwrap();
    plain.migrate().await.unwrap();
    plain.migrate().await.unwrap();
    assert!(
        plain
            .session(
                &key,
                before["session"]["credential"].as_str().unwrap(),
                "inspect"
            )
            .await
            .is_ok()
    );
    let checksum: String = db
        .query_one("SELECT digest FROM kanche_migrations WHERE version=2", &[])
        .await
        .unwrap()
        .get(0);
    db.execute(
        "UPDATE kanche_migrations SET digest='invalid' WHERE version=2",
        &[],
    )
    .await
    .unwrap();
    assert!(plain.migrate().await.is_err());
    db.execute(
        "UPDATE kanche_migrations SET digest=$1 WHERE version=2",
        &[&checksum],
    )
    .await
    .unwrap();
    plain.migrate().await.unwrap();
    assert!(
        request(
            &plain,
            ("POST", "/v1/hosted/transactions"),
            Some(&key),
            None,
            None,
            Some("application/json"),
            "{}".into()
        )
        .await
        .status
            == 404
    );
    assert!(
        !plain.application(&key).await.unwrap()["features"]
            .as_array()
            .unwrap()
            .contains(&json!("hosted_login"))
    );
    let s = plain.clone().with_hosted_origin(ORIGIN).unwrap();
    let second = Store::new(url.as_str())
        .unwrap()
        .with_hosted_origin(ORIGIN)
        .unwrap();
    assert!(
        s.application(&key).await.unwrap()["features"]
            .as_array()
            .unwrap()
            .contains(&json!("hosted_login"))
    );
    s.provision(provision(
        "hosted_tenant",
        "hosted_second_app",
        "hosted_user",
        &key2,
        &password,
    ))
    .await
    .unwrap();
    s.provision(provision(
        "hosted_foreign",
        "hosted_foreign_app",
        "hosted_foreign_user",
        &foreign,
        &password,
    ))
    .await
    .unwrap();
    s.register_callback(registration("hosted_app"))
        .await
        .unwrap();
    s.register_callback(registration("hosted_app"))
        .await
        .unwrap();
    s.register_callback(registration("hosted_second_app"))
        .await
        .unwrap();
    assert!(
        s.register_callback(HostedRegistration {
            application_id: "hosted_app".into(),
            redirect_uri: CALLBACK.into(),
            development: false
        })
        .await
        .is_err()
    );
    let verifier = secret();
    let state = secret();
    let mut wrong_callback = create_input(&state, &verifier);
    wrong_callback.redirect_uri = "https://evil.example/callback".into();
    assert!(
        s.create_hosted_transaction(&key, wrong_callback)
            .await
            .is_err()
    );
    let result=request(&s,("POST", "/v1/hosted/transactions"),Some(&key),None,None,Some("application/json"),json!({"redirect_uri":CALLBACK,"state":state,"code_challenge":challenge(&verifier),"code_challenge_method":"S256","unexpected":true}).to_string()).await;
    assert!(result.status == 400);
    assert!(!result.body.contains(&key));
    let transaction = s
        .create_hosted_transaction(&key, create_input(&state, &verifier))
        .await
        .unwrap();
    let first_handle = handle(&transaction);
    assert!(
        request(
            &s,
            (
                "GET",
                &format!("/hosted/login?transaction={first_handle}&transaction={first_handle}")
            ),
            None,
            None,
            None,
            None,
            String::new()
        )
        .await
        .status
            == 400
    );
    let (cookie, csrf) = get_form(&s, &first_handle, None).await;
    assert!(
        request(
            &s,
            ("GET", &format!("/hosted/login?transaction={first_handle}")),
            None,
            None,
            None,
            None,
            String::new()
        )
        .await
        .status
            == 400
    );
    assert!(
        password_response(
            &s,
            &first_handle,
            &cookie,
            &secret(),
            "hosted_user@example.invalid",
            &password
        )
        .await
        .status
            == 403
    );
    for (content_type, body, expected) in [
        ("text/plain", "invalid".into(), 415),
        ("application/x-www-form-urlencoded", "x".repeat(32769), 413),
    ] {
        let rejected = request(
            &s,
            ("POST", "/hosted/login"),
            None,
            Some(&cookie),
            Some(ORIGIN),
            Some(content_type),
            body,
        )
        .await;
        assert!(rejected.status == expected);
        assert!(rejected.headers["referrer-policy"] == "same-origin");
        assert!(rejected.headers["x-content-type-options"] == "nosniff");
        assert!(
            rejected.headers["content-security-policy"]
                .to_str()
                .unwrap()
                .contains("frame-ancestors 'none'")
        );
    }
    // Malformed UTF8 must reject before consuming password attempts or changing password bytes.
    let malformed = format!(
        "transaction={first_handle}&csrf_token={csrf}&email=hosted_user%40example.invalid&password=%FF"
    );
    let malformed = request(
        &s,
        ("POST", "/hosted/login"),
        None,
        Some(&cookie),
        Some(ORIGIN),
        Some("application/x-www-form-urlencoded"),
        malformed,
    )
    .await;
    assert!(malformed.status == 400);
    assert!(
        db.query_one(
            "SELECT count(*) FROM abuse_buckets WHERE bucket LIKE 'account:%'",
            &[]
        )
        .await
        .unwrap()
        .get::<_, i64>(0)
            == 1
    ); // only the prior app-owned login
    let cross_origin = request(
        &s,
        ("POST", "/hosted/login"),
        None,
        Some(&cookie),
        Some("https://evil.example"),
        Some("application/x-www-form-urlencoded"),
        form(
            &first_handle,
            &csrf,
            "hosted_user@example.invalid",
            &password,
        ),
    )
    .await;
    assert!(cross_origin.status == 403);
    let bad = password_response(
        &s,
        &first_handle,
        &cookie,
        &csrf,
        "hosted_user@example.invalid",
        "incorrect synthetic password",
    )
    .await;
    assert!(bad.status == 401);
    assert!(
        bad.headers["content-security-policy"]
            .to_str()
            .unwrap()
            .contains(&format!("form-action 'self' {CALLBACK};"))
    );
    assert!(
        bad.headers["content-type"]
            .to_str()
            .unwrap()
            .starts_with("text/html")
    );
    assert!(!bad.body.contains("hosted_user@example.invalid"));
    assert!(!bad.body.contains("incorrect synthetic password"));
    // Reload rotates CSRF; an old form fails safely.
    let (_, fresh_csrf) = get_form(&s, &first_handle, Some(&cookie)).await;
    assert!(
        password_response(
            &s,
            &first_handle,
            &cookie,
            &csrf,
            "hosted_user@example.invalid",
            &password
        )
        .await
        .status
            == 403
    );
    let before_sessions: i64 = db
        .query_one("SELECT count(*) FROM sessions", &[])
        .await
        .unwrap()
        .get(0);
    let completed = password_response(
        &s,
        &first_handle,
        &cookie,
        &fresh_csrf,
        "hosted_user@example.invalid",
        &password,
    )
    .await;
    assert!(completed.status == 303);
    assert!(completed.headers["referrer-policy"] == "no-referrer");
    assert!(
        db.query_one("SELECT count(*) FROM sessions", &[])
            .await
            .unwrap()
            .get::<_, i64>(0)
            == before_sessions
    );
    let code = Url::parse(completed.headers["location"].to_str().unwrap())
        .unwrap()
        .query_pairs()
        .find(|(k, _)| k == "code")
        .unwrap()
        .1
        .into_owned();
    for wrong_key in [&key2, &foreign] {
        assert!(
            s.exchange_hosted(wrong_key, exchange_input(&code, &verifier))
                .await
                .is_err()
        );
    }
    assert!(
        s.exchange_hosted(&key, exchange_input(&code, &secret()))
            .await
            .is_err()
    );
    let mut wrong = exchange_input(&code, &verifier);
    wrong.redirect_uri = "https://evil.example/callback".into();
    assert!(s.exchange_hosted(&key, wrong).await.is_err());
    let (a, b) = tokio::join!(
        s.exchange_hosted(&key, exchange_input(&code, &verifier)),
        second.exchange_hosted(&key, exchange_input(&code, &verifier))
    );
    assert!(
        usize::from(a.is_ok()) + usize::from(b.is_ok()) == 1,
        "handoff must issue exactly one session"
    );
    let grant = match (a, b) {
        (Ok(v), _) | (_, Ok(v)) => v,
        _ => unreachable!(),
    };
    assert!(
        s.session(
            &key,
            grant["session"]["credential"].as_str().unwrap(),
            "inspect"
        )
        .await
        .is_ok()
    );
    assert!(
        s.exchange_hosted(&key, exchange_input(&code, &verifier))
            .await
            .is_err()
    );
    assert!(
        password_response(
            &s,
            &first_handle,
            &cookie,
            &fresh_csrf,
            "hosted_user@example.invalid",
            &password
        )
        .await
        .status
            == 400
    );
    // Reset synthetic test buckets between independent state-machine cases.
    db.execute("DELETE FROM abuse_buckets", &[]).await.unwrap();
    let (removed_code, _, _, _) = authenticated_code(&s, &key, &password, &verifier).await;
    s.maintenance("remove-enrollment", "hosted_app", "hosted_user")
        .await
        .unwrap();
    db.execute(
        "UPDATE enrollments SET active=true WHERE app_id='hosted_app' AND user_id='hosted_user'",
        &[],
    )
    .await
    .unwrap();
    assert!(
        s.exchange_hosted(&key, exchange_input(&removed_code, &verifier))
            .await
            .is_err()
    );
    let (expired_code, _, _, _) = authenticated_code(&s, &key, &password, &verifier).await;
    db.execute("UPDATE hosted_transactions SET code_expires_at=floor(extract(epoch FROM clock_timestamp()))::bigint WHERE code_digest=$1",&[&digest(&expired_code)]).await.unwrap();
    assert!(
        s.exchange_hosted(&key, exchange_input(&expired_code, &verifier))
            .await
            .is_err()
    );
    let tx = s
        .create_hosted_transaction(&key, create_input(&secret(), &verifier))
        .await
        .unwrap();
    let expired_handle = handle(&tx);
    db.execute("UPDATE hosted_transactions SET expires_at=floor(extract(epoch FROM clock_timestamp()))::bigint WHERE handle_digest=$1",&[&digest(&expired_handle)]).await.unwrap();
    assert!(
        request(
            &s,
            (
                "GET",
                &format!("/hosted/login?transaction={expired_handle}")
            ),
            None,
            None,
            None,
            None,
            String::new()
        )
        .await
        .status
            == 400
    );
    let (deregistered_code, _, _, _) = authenticated_code(&s, &key, &password, &verifier).await;
    s.remove_callback(registration("hosted_app")).await.unwrap();
    s.register_callback(registration("hosted_app"))
        .await
        .unwrap();
    assert!(
        s.exchange_hosted(&key, exchange_input(&deregistered_code, &verifier))
            .await
            .is_err()
    );
    db.execute("DELETE FROM abuse_buckets", &[]).await.unwrap();
    let transaction = s
        .create_hosted_transaction(&key, create_input(&secret(), &verifier))
        .await
        .unwrap();
    let concurrent_handle = handle(&transaction);
    let (browser, csrf) = get_form(&s, &concurrent_handle, None).await;
    let (first, second_result) = tokio::join!(
        password_response(
            &s,
            &concurrent_handle,
            &browser,
            &csrf,
            "hosted_user@example.invalid",
            &password
        ),
        password_response(
            &second,
            &concurrent_handle,
            &browser,
            &csrf,
            "hosted_user@example.invalid",
            &password
        )
    );
    assert!(
        usize::from(first.status == 303) + usize::from(second_result.status == 303) == 1,
        "concurrent password completion must create only one handoff"
    );
    let (global_code, _, _, _) = authenticated_code(&s, &key, &password, &verifier).await;
    s.maintenance("revoke-all", "", "").await.unwrap();
    assert!(
        s.exchange_hosted(&key, exchange_input(&global_code, &verifier))
            .await
            .is_err()
    );
    s.maintenance("revoke-app-sessions", "hosted_app", "")
        .await
        .unwrap();
    assert!(
        db.query_one("SELECT active FROM applications WHERE id='hosted_app'", &[])
            .await
            .unwrap()
            .get::<_, bool>(0)
    );
    assert_eq!(
        db.query_one(
            "SELECT count(*) FROM sessions WHERE app_id='hosted_app' AND NOT revoked",
            &[]
        )
        .await
        .unwrap()
        .get::<_, i64>(0),
        0
    );
    db.execute("DELETE FROM abuse_buckets", &[]).await.unwrap();
    db.execute("INSERT INTO abuse_buckets VALUES ('hosted-create:hosted_app', floor(extract(epoch FROM clock_timestamp()))::bigint+60, 30)", &[]).await.unwrap();
    let limited = request(
        &s,
        ("POST", "/v1/hosted/transactions"),
        Some(&key),
        None,
        None,
        Some("application/json"),
        json!({"redirect_uri": CALLBACK, "state": secret(), "code_challenge": challenge(&verifier), "code_challenge_method": "S256"}).to_string(),
    )
    .await;
    assert_eq!(limited.status, 429);
    assert_eq!(limited.headers["retry-after"], "60");
    assert_eq!(
        serde_json::from_str::<Value>(&limited.body).unwrap()["code"],
        "rate_limited"
    );
    s.maintenance("cleanup", "", "").await.unwrap();
    assert!(db.query_one("SELECT count(*) FROM hosted_transactions WHERE invalidated OR consumed OR expires_at<=floor(extract(epoch FROM clock_timestamp()))::bigint OR code_expires_at<=floor(extract(epoch FROM clock_timestamp()))::bigint",&[]).await.unwrap().get::<_,i64>(0)==0);
    println!(
        "Hosted migration, registration, browser/CSRF/PKCE binding, expiry, replay, concurrency, eligibility/removal and cleanup checks passed"
    );
}
