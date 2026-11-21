// SPDX-License-Identifier: MPL-2.0
use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use kanche::{
    app_with_store,
    identity::{Provision, SignIn, Store, secret},
};
use serde_json::{Value, json};
use tower::ServiceExt;
async fn call(
    store: &Store,
    method: &str,
    path: &str,
    key: &str,
    token: &str,
    body: Option<Value>,
) -> (u16, Value) {
    let response = app_with_store(Some(store.clone()))
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("authorization", format!("Bearer {key}"))
                .header("kanche-session", token)
                .header("content-type", "application/json")
                .body(Body::from(body.map_or(String::new(), |v| v.to_string())))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status().as_u16();
    let request_id = response.headers()["x-request-id"]
        .to_str()
        .unwrap()
        .to_owned();
    assert_eq!(response.headers()["cache-control"], "no-store");
    let bytes = to_bytes(response.into_body(), 32768).await.unwrap();
    let v: Value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    if status >= 400 {
        assert_eq!(v["request_id"], request_id);
        assert!(!v.to_string().contains(key));
        if !token.is_empty() {
            assert!(!v.to_string().contains(token));
        }
    }
    (status, v)
}
fn p(t: &str, a: &str, u: &str, key: &str, password: &str) -> Provision {
    Provision {
        tenant_id: t.into(),
        application_id: a.into(),
        user_id: u.into(),
        email: format!("{u}@example.invalid"),
        password: password.into(),
        application_secret: key.into(),
    }
}
async fn login(s: &Store, key: &str, u: &str, password: &str) -> Value {
    let (status, v) = call(
        s,
        "POST",
        "/v1/auth/sign-ins",
        key,
        "",
        Some(json!({"email":format!("{u}@example.invalid"),"password":password})),
    )
    .await;
    assert_eq!(status, 200);
    v
}
#[tokio::test]
#[ignore = "requires disposable PostgreSQL; run make test-integration"]
async fn app_owned_invariants() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::WARN)
        .try_init();
    let url = std::env::var("KANCHE_TEST_DATABASE_URL").expect("disposable test database required");
    let s = Store::new(&url).unwrap();
    s.migrate().await.unwrap();
    s.migrate().await.unwrap();
    let other = Store::new(&url).unwrap();
    let key = secret();
    let key2 = secret();
    let foreign = secret();
    let password = secret();
    s.provision(p("t1", "a1", "u1", &key, &password))
        .await
        .unwrap();
    s.provision(p("t1", "a1", "u1", &key, &password))
        .await
        .unwrap();
    s.provision(p("t1", "a2", "u1", &key2, &password))
        .await
        .unwrap();
    s.provision(p("t2", "a3", "u2", &foreign, &password))
        .await
        .unwrap();
    assert!(
        s.provision(p("t2", "a1", "u2", &foreign, &password))
            .await
            .is_err()
    );
    let mut db = s.connect().await.unwrap();
    for (app, user, tenant, verified, active) in [
        ("a1", "unenrolled", "t1", true, true),
        ("a1", "unverified", "t1", false, true),
        ("a1", "disabled", "t1", true, false),
    ] {
        s.provision(p(tenant, app, user, &key, &password))
            .await
            .unwrap();
        if user == "unenrolled" {
            db.execute("DELETE FROM enrollments WHERE user_id=$1", &[&user])
                .await
                .unwrap();
        }
        db.execute(
            "UPDATE users SET verified=$1,active=$2 WHERE id=$3",
            &[&verified, &active, &user],
        )
        .await
        .unwrap();
        assert_eq!(
            call(
                &s,
                "POST",
                "/v1/auth/sign-ins",
                &key,
                "",
                Some(json!({"email":format!("{user}@example.invalid"),"password":password}))
            )
            .await
            .0,
            401
        );
    }
    for (email, pw) in [
        ("missing@example.invalid", password.as_str()),
        ("u1@example.invalid", "wrong password"),
    ] {
        let (status, v) = call(
            &s,
            "POST",
            "/v1/auth/sign-ins",
            &key,
            "",
            Some(json!({"email":email,"password":pw})),
        )
        .await;
        assert_eq!(status, 401);
        assert_eq!(v["code"], "authentication_failed");
    }
    assert_eq!(
        call(&s, "GET", "/v1/application", &foreign, "", None)
            .await
            .1["tenant_id"],
        "t2"
    );
    assert_eq!(
        call(&s, "GET", "/v1/application", &secret(), "", None)
            .await
            .0,
        401
    );
    let grant = login(&s, &key, "u1", &password).await;
    let token = grant["session"]["credential"].as_str().unwrap();
    let device = login(&s, &key, "u1", &password).await;
    let device_token = device["session"]["credential"].as_str().unwrap();
    let app2 = login(&s, &key2, "u1", &password).await;
    let app2_token = app2["session"]["credential"].as_str().unwrap();
    for k in [&key2, &foreign] {
        assert_eq!(
            call(&s, "GET", "/v1/sessions/current", k, token, None)
                .await
                .0,
            401
        );
        assert_eq!(
            call(&s, "POST", "/v1/sessions/current/revoke", k, token, None)
                .await
                .0,
            204
        );
    }
    let (status, v) = call(&s, "GET", "/v1/sessions/current", &key, token, None).await;
    assert_eq!(status, 200);
    assert!(v["session"].get("credential").is_none());
    assert_eq!(v["session"], {
        let mut v = grant["session"].clone();
        v.as_object_mut().unwrap().remove("credential");
        v
    });
    // Two independently configured instances renew one stable credential, without extending absolute.
    let (r1, r2) = tokio::join!(
        s.session(&key, token, "renew"),
        other.session(&key, token, "renew")
    );
    assert!(r1.is_ok() && r2.is_ok());
    assert_eq!(
        r1.unwrap()["session"]["absolute_expires_at"],
        grant["session"]["absolute_expires_at"]
    );
    // Lost renewal response is recoverable by using the original bearer again.
    assert!(s.session(&key, token, "renew").await.is_ok());
    assert!(other.session(&key, token, "inspect").await.is_ok());
    db.execute("UPDATE sessions SET absolute_expires_at=floor(extract(epoch FROM clock_timestamp()))::bigint+60,idle_expires_at=floor(extract(epoch FROM clock_timestamp()))::bigint+30 WHERE digest=$1",&[&kanche::identity::digest(token)]).await.unwrap();
    let capped = s.session(&key, token, "renew").await.unwrap();
    assert_eq!(
        capped["session"]["idle_expires_at"],
        capped["session"]["absolute_expires_at"]
    );
    // Exact boundary is invalid; no grace and renewal cannot revive expired rows.
    db.execute("UPDATE sessions SET idle_expires_at=floor(extract(epoch FROM clock_timestamp()))::bigint WHERE digest=$1",&[&kanche::identity::digest(token)]).await.unwrap();
    assert!(s.session(&key, token, "inspect").await.is_err());
    assert!(s.session(&key, token, "renew").await.is_err());
    // A different live device remains active when the current device signs out.
    let survivor = login(&s, &key, "u1", &password).await;
    let survivor_token = survivor["session"]["credential"].as_str().unwrap();

    assert_eq!(
        call(
            &s,
            "POST",
            "/v1/sessions/current/revoke",
            &key,
            device_token,
            None
        )
        .await
        .0,
        204
    );
    assert!(s.session(&key, device_token, "inspect").await.is_err());
    assert!(s.session(&key2, app2_token, "inspect").await.is_ok());
    assert!(s.session(&key, survivor_token, "inspect").await.is_ok());
    // Current user eligibility is checked on every request (no positive cache).
    db.execute("UPDATE users SET active=false WHERE id='u1'", &[])
        .await
        .unwrap();
    assert!(s.session(&key, survivor_token, "inspect").await.is_err());
    db.execute("UPDATE users SET active=true WHERE id='u1'", &[])
        .await
        .unwrap();
    // Database backward clock simulation: never return a lower renewal deadline.
    let before = s.session(&key, survivor_token, "inspect").await.unwrap();
    db.execute(
        "UPDATE sessions SET idle_expires_at=idle_expires_at+60 WHERE digest=$1",
        &[&kanche::identity::digest(survivor_token)],
    )
    .await
    .unwrap();
    let renewed = s.session(&key, survivor_token, "renew").await.unwrap();
    assert!(
        renewed["session"]["idle_expires_at"].as_i64().unwrap()
            >= before["session"]["idle_expires_at"].as_i64().unwrap() + 60
    );
    // Concurrent revoke/renew cannot resurrect the revoked bearer across instances.
    let (revoked, _renewed) = tokio::join!(
        s.session(&key, survivor_token, "revoke"),
        other.session(&key, survivor_token, "renew")
    );
    assert!(revoked.is_ok());
    assert!(s.session(&key, survivor_token, "inspect").await.is_err());
    // Removal/regrant must not revive old sessions.
    s.maintenance("remove-enrollment", "a2", "u1")
        .await
        .unwrap();
    db.execute(
        "UPDATE enrollments SET active=true WHERE app_id='a2' AND user_id='u1'",
        &[],
    )
    .await
    .unwrap();
    assert!(s.session(&key2, app2_token, "inspect").await.is_err());
    // Rate limiting persists through independent instances, including unknown identities.
    for _ in 0..5 {
        let _ = other
            .sign_in(
                &key2,
                SignIn {
                    email: "unknown@example.invalid".into(),
                    password: password.clone(),
                },
            )
            .await;
    }
    assert_eq!(
        s.sign_in(
            &key2,
            SignIn {
                email: "unknown@example.invalid".into(),
                password: password.clone()
            }
        )
        .await
        .unwrap_err(),
        "rate_limited"
    );
    // Failure to acquire the authority lock is unavailable, not invalid session.
    let tx = Store::begin(&mut db).await.unwrap();
    let err = s.session(&key2, app2_token, "inspect").await.unwrap_err();
    assert_eq!(err, "temporarily_unavailable");
    tx.rollback().await.unwrap();
    assert!(s.application(&key).await.is_ok());
    db.execute("DELETE FROM abuse_buckets", &[]).await.unwrap();
    // App and user administrative removal revoke rows before a later re-enable.
    let before_disable = login(&s, &key, "u1", &password).await;
    let before_disable_token = before_disable["session"]["credential"].as_str().unwrap();
    s.maintenance("disable-user", "", "u1").await.unwrap();
    db.execute("UPDATE users SET active=true WHERE id='u1'", &[])
        .await
        .unwrap();
    assert!(
        s.session(&key, before_disable_token, "inspect")
            .await
            .is_err()
    );
    let before_app_disable = login(&s, &key, "u1", &password).await;
    let before_app_token = before_app_disable["session"]["credential"]
        .as_str()
        .unwrap();
    s.maintenance("disable-app", "a1", "").await.unwrap();
    db.execute("UPDATE applications SET active=true WHERE id='a1'", &[])
        .await
        .unwrap();
    assert!(s.session(&key, before_app_token, "inspect").await.is_err());
    // Absolute boundary on a live session, even when idle equals absolute.
    let absolute_grant = login(&s, &key2, "u1", &password).await;
    let absolute_token = absolute_grant["session"]["credential"].as_str().unwrap();
    db.execute("UPDATE sessions SET idle_expires_at=floor(extract(epoch FROM clock_timestamp()))::bigint,absolute_expires_at=floor(extract(epoch FROM clock_timestamp()))::bigint WHERE digest=$1",&[&kanche::identity::digest(absolute_token)]).await.unwrap();
    assert!(s.session(&key2, absolute_token, "renew").await.is_err());
    s.maintenance("remove-key", &kanche::identity::digest(&foreign), "")
        .await
        .unwrap();
    assert!(s.application(&foreign).await.is_err());
    s.maintenance("revoke-all", "", "").await.unwrap();
    s.maintenance("cleanup", "", "").await.unwrap();
    assert_eq!(
        db.query_one("SELECT count(*) FROM sessions", &[])
            .await
            .unwrap()
            .get::<_, i64>(0),
        0
    );
    // Transport rejection uses safe problems.
    assert_eq!(
        call(
            &s,
            "POST",
            "/v1/auth/sign-ins",
            &key,
            "",
            Some(json!({"email":"x@y","password":"x","unexpected":true}))
        )
        .await
        .0,
        400
    );
    println!(
        "PostgreSQL app isolation, login, expiry, renewal, replay, revocation, contention recovery, cleanup and migration checks passed"
    );
}
