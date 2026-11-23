// SPDX-License-Identifier: MPL-2.0
//! Native hosted password login; no OAuth/OIDC or shared-SSO claim.
use super::*;
use axum::{
    Json, Router,
    body::Bytes,
    extract::{
        DefaultBodyLimit, Query, RawQuery, State,
        rejection::{BytesRejection, QueryRejection},
    },
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use url::Url;

pub(crate) struct HostedConfig {
    origin: String,
    development: bool,
}
#[derive(Clone)]
pub(crate) struct SafeHostedHtml;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostedRegistration {
    pub application_id: String,
    pub redirect_uri: String,
    #[serde(default)]
    pub development: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostedTransaction {
    pub redirect_uri: String,
    pub state: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostedExchange {
    pub code: String,
    pub code_verifier: String,
    pub redirect_uri: String,
}
fn hex_token(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn verifier(value: &str) -> bool {
    (43..=128).contains(&value.len())
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-._~".contains(&b))
}
fn challenge(value: &str) -> bool {
    value.len() == 43
        && URL_SAFE_NO_PAD
            .decode(value)
            .is_ok_and(|v| v.len() == 32 && URL_SAFE_NO_PAD.encode(v) == value)
}
fn proof_challenge(value: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(value.as_bytes()))
}
fn canonical_url(value: &str, development: bool, callback: bool) -> Result<Url, Failure> {
    if value.len() > 2048
        || !value.is_ascii()
        || value
            .bytes()
            .any(|b| b.is_ascii_control() || b.is_ascii_whitespace())
        || value.contains(['\\', '%', '*', '?', '#'])
    {
        return Err("invalid_redirect_uri");
    }
    let url = Url::parse(value).map_err(|_| "invalid_redirect_uri")?;
    let loopback = match url.host() {
        Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
        Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
        Some(url::Host::Domain("localhost")) => true,
        _ => false,
    };
    if !url.username().is_empty()
        || url.password().is_some()
        || url.host_str().is_none()
        || !(url.scheme() == "https" || (development && loopback && url.scheme() == "http"))
        || (callback && (url.path() == "/" || !url.path().starts_with('/')))
    {
        return Err("invalid_redirect_uri");
    }
    if url
        .origin()
        .ascii_serialization()
        .bytes()
        .any(|byte| !byte.is_ascii_alphanumeric() && !b":/.-[]".contains(&byte))
    {
        return Err("invalid_redirect_uri");
    }
    if callback {
        if url.as_str() != value {
            return Err("invalid_redirect_uri");
        }
    } else if url.origin().ascii_serialization() != value || url.path() != "/" {
        return Err("invalid_redirect_uri");
    }
    Ok(url)
}
#[derive(Clone)]
struct LoginForm {
    handle: String,
    csrf: String,
    callback_source: String,
}
impl LoginForm {
    fn new(handle: &str, csrf: &str, redirect: &str) -> Result<Self, Failure> {
        if !hex_token(handle) || !hex_token(csrf) {
            return Err("invalid_handoff");
        }
        let callback = canonical_url(redirect, true, true)?;
        let mut source = callback.origin().ascii_serialization();
        for byte in callback.path().bytes() {
            if byte.is_ascii_alphanumeric() || b"-._~/".contains(&byte) {
                source.push(byte as char)
            } else {
                use std::fmt::Write;
                write!(&mut source, "%{byte:02X}").expect("writing to String");
            }
        }
        Ok(Self {
            handle: handle.to_owned(),
            csrf: csrf.to_owned(),
            callback_source: source,
        })
    }
}
struct HostedPasswordError {
    code: Failure,
    form: Option<LoginForm>,
}
impl From<Failure> for HostedPasswordError {
    fn from(code: Failure) -> Self {
        Self { code, form: None }
    }
}
impl LoginForm {
    fn failure(&self, code: Failure) -> HostedPasswordError {
        HostedPasswordError {
            code,
            form: Some(self.clone()),
        }
    }
}
impl Store {
    /// Explicit origin opt-in. HTTP is restricted to literal loopback IPs/localhost.
    pub fn with_hosted_origin(mut self, origin: &str) -> Result<Self, Failure> {
        let url = canonical_url(origin, true, false).map_err(|_| "Invalid KANCHE_HOSTED_ORIGIN")?;
        self.hosted = Some(Arc::new(HostedConfig {
            origin: origin.to_owned(),
            development: url.scheme() == "http",
        }));
        Ok(self)
    }
    pub(crate) fn hosted_enabled(&self) -> bool {
        self.hosted.is_some()
    }
    pub async fn register_callback(&self, input: HostedRegistration) -> Result<(), Failure> {
        canonical_url(&input.redirect_uri, input.development, true)?;
        if input.application_id.is_empty() || input.application_id.len() > 128 {
            return Err("invalid_request");
        }
        let mut client = self.connect().await?;
        let tx = Self::begin(&mut client).await?;
        if tx
            .query_opt(
                "SELECT id FROM applications WHERE id=$1",
                &[&input.application_id],
            )
            .await
            .map_err(db_error)?
            .is_none()
        {
            return Err("invalid_request");
        }
        tx.execute(
            "INSERT INTO hosted_callbacks VALUES($1,$2) ON CONFLICT DO NOTHING",
            &[&input.application_id, &input.redirect_uri],
        )
        .await
        .map_err(db_error)?;
        tx.commit().await.map_err(db_error)
    }
    pub async fn remove_callback(&self, input: HostedRegistration) -> Result<(), Failure> {
        canonical_url(&input.redirect_uri, input.development, true)?;
        let mut client = self.connect().await?;
        let tx = Self::begin(&mut client).await?;
        // The FK deletes pending and successful-password handoffs atomically.
        tx.execute(
            "DELETE FROM hosted_callbacks WHERE app_id=$1 AND redirect_uri=$2",
            &[&input.application_id, &input.redirect_uri],
        )
        .await
        .map_err(db_error)?;
        tx.commit().await.map_err(db_error)
    }
    pub async fn create_hosted_transaction(
        &self,
        key: &str,
        input: HostedTransaction,
    ) -> Result<serde_json::Value, Failure> {
        let config = self.hosted.as_ref().ok_or("invalid_request")?;
        if !hex_token(&input.state)
            || !challenge(&input.code_challenge)
            || input.code_challenge_method != "S256"
        {
            return Err("invalid_request");
        }
        // Registration supplies the development decision, never this public input.
        let mut client = self.connect().await?;
        let tx = Self::begin(&mut client).await?;
        let scope = Self::scope(&tx, key).await?;
        if tx
            .query_opt(
                "SELECT app_id FROM hosted_callbacks WHERE app_id=$1 AND redirect_uri=$2",
                &[&scope.app, &input.redirect_uri],
            )
            .await
            .map_err(db_error)?
            .is_none()
        {
            return Err("invalid_redirect_uri");
        }
        let timestamp = now(&tx).await?;
        if let Err(error) = rate(
            &tx,
            &format!("hosted-create:{}", scope.app),
            timestamp,
            60,
            30,
        )
        .await
        {
            if error == "rate_limited" {
                tx.commit().await.map_err(db_error)?;
            }
            return Err(if error == "rate_limited" {
                "rate_limited_short"
            } else {
                error
            });
        }
        let handle = secret();
        let expires = timestamp + 300;
        tx.execute("INSERT INTO hosted_transactions(handle_digest,tenant_id,app_id,redirect_uri,state,code_challenge,expires_at) VALUES($1,$2,$3,$4,$5,$6,$7)",&[&digest(&handle),&scope.tenant,&scope.app,&input.redirect_uri,&input.state,&input.code_challenge,&expires]).await.map_err(db_error)?;
        tx.commit().await.map_err(db_error)?;
        Ok(
            serde_json::json!({"authorization_url":format!("{}/hosted/login?transaction={handle}",config.origin),"expires_at":expires}),
        )
    }
    async fn hosted_page(
        &self,
        handle: &str,
        browser: Option<&str>,
    ) -> Result<(String, LoginForm), Failure> {
        if !hex_token(handle) || browser.is_some_and(|v| !hex_token(v)) {
            return Err("invalid_handoff");
        }
        let _page_permit = self
            .hosted_pages
            .clone()
            .try_acquire_owned()
            .map_err(|_| "temporarily_unavailable")?;
        let mut client = self.connect().await?;
        let tx = Self::begin(&mut client).await?;
        let timestamp = now(&tx).await?;
        let pending = pending(&tx, handle, timestamp).await?;
        let browser = match (pending.browser, browser) {
            (Some(expected), Some(value)) if expected == digest(value) => value.to_owned(),
            (Some(_), _) => return Err("invalid_handoff"),
            (None, Some(value)) => value.to_owned(),
            (None, None) => secret(),
        };
        let csrf = secret();
        tx.execute("UPDATE hosted_transactions SET browser_digest=$1,csrf_digest=$2 WHERE handle_digest=$3",&[&digest(&browser),&digest(&csrf),&digest(handle)]).await.map_err(db_error)?;
        tx.commit().await.map_err(db_error)?;
        Ok((browser, LoginForm::new(handle, &csrf, &pending.redirect)?))
    }
    async fn complete_hosted_password(
        &self,
        handle: &str,
        csrf: &str,
        browser: &str,
        input: SignIn,
    ) -> Result<String, HostedPasswordError> {
        if !hex_token(handle) || !hex_token(csrf) || !hex_token(browser) {
            return Err("invalid_handoff".into());
        }
        validate_password_input(&input)?;
        let permit = self
            .admission
            .clone()
            .try_acquire_owned()
            .map_err(|_| "temporarily_unavailable")?;
        let mut client = self.connect().await?;
        let tx = Self::begin(&mut client).await?;
        let timestamp = now(&tx).await?;
        let first = pending(&tx, handle, timestamp).await?;
        bound(&first, browser, csrf)?;
        let form = LoginForm::new(handle, csrf, &first.redirect)?;
        let prepared = match self.prepare_password(&tx, &first.scope, input).await {
            Ok(prepared) => prepared,
            Err(error @ "rate_limited") => {
                tx.commit().await.map_err(db_error)?;
                return Err(form.failure(error));
            }
            Err(error) => return Err(error.into()),
        };
        tx.commit().await.map_err(db_error)?;
        let proof = prepared
            .verify(permit)
            .await
            .map_err(|code| form.failure(code))?;
        let tx = Self::begin(&mut client).await?;
        let timestamp = now(&tx).await?;
        let current = pending(&tx, handle, timestamp).await?;
        bound(&current, browser, csrf)?;
        if current.scope.app != first.scope.app
            || current.scope.tenant != first.scope.tenant
            || !valid_proof(&tx, &current.scope, &proof).await?
        {
            return Err(form.failure("authentication_failed"));
        }
        let code = secret();
        let expires = (timestamp + 60).min(current.expires);
        tx.execute("UPDATE hosted_transactions SET user_id=$1,code_digest=$2,code_expires_at=$3 WHERE handle_digest=$4",&[&proof.user,&digest(&code),&expires,&digest(handle)]).await.map_err(db_error)?;
        let mut redirect = Url::parse(&current.redirect).map_err(|_| "invalid_handoff")?;
        redirect
            .query_pairs_mut()
            .append_pair("code", &code)
            .append_pair("state", &current.state);
        tx.commit().await.map_err(db_error)?;
        Ok(redirect.into())
    }
    pub async fn exchange_hosted(
        &self,
        key: &str,
        input: HostedExchange,
    ) -> Result<serde_json::Value, Failure> {
        if !self.hosted_enabled() {
            return Err("invalid_request");
        }
        if !hex_token(&input.code) || !verifier(&input.code_verifier) {
            return Err("invalid_handoff");
        }
        let mut client = self.connect().await?;
        let tx = Self::begin(&mut client).await?;
        let scope = Self::scope(&tx, key).await?;
        let timestamp = now(&tx).await?;
        let row = tx
            .query_opt(
                "SELECT h.user_id,h.code_challenge FROM hosted_transactions h
             JOIN hosted_callbacks c ON c.app_id=h.app_id AND c.redirect_uri=h.redirect_uri
             WHERE h.code_digest=$1 AND h.tenant_id=$2 AND h.app_id=$3 AND h.redirect_uri=$4
             AND h.expires_at>$5 AND h.code_expires_at>$5 AND NOT h.consumed AND NOT h.invalidated",
                &[
                    &digest(&input.code),
                    &scope.tenant,
                    &scope.app,
                    &input.redirect_uri,
                    &timestamp,
                ],
            )
            .await
            .map_err(db_error)?
            .ok_or("invalid_handoff")?;
        let user: String = row.get(0);
        if row.get::<_, String>(1) != proof_challenge(&input.code_verifier)
            || !eligible(&tx, &scope, &user).await?
        {
            return Err("invalid_handoff");
        }
        let grant = issue(&tx, &scope, &user).await?;
        tx.execute(
            "UPDATE hosted_transactions SET consumed=true WHERE code_digest=$1",
            &[&digest(&input.code)],
        )
        .await
        .map_err(db_error)?;
        tx.commit().await.map_err(db_error)?;
        Ok(grant)
    }
}
struct Pending {
    scope: Scope,
    redirect: String,
    state: String,
    expires: i64,
    browser: Option<String>,
    csrf: Option<String>,
}
async fn pending(tx: &Transaction<'_>, handle: &str, timestamp: i64) -> Result<Pending, Failure> {
    let row=tx.query_opt(
        "SELECT h.tenant_id,h.app_id,h.redirect_uri,h.state,h.expires_at,h.browser_digest,h.csrf_digest
         FROM hosted_transactions h JOIN applications a ON a.id=h.app_id AND a.tenant_id=h.tenant_id
         JOIN tenants t ON t.id=a.tenant_id
         JOIN hosted_callbacks c ON c.app_id=h.app_id AND c.redirect_uri=h.redirect_uri
         WHERE h.handle_digest=$1 AND h.expires_at>$2 AND NOT h.consumed AND NOT h.invalidated
         AND h.code_digest IS NULL AND a.active AND t.active",
        &[&digest(handle),&timestamp]).await.map_err(db_error)?.ok_or("invalid_handoff")?;
    Ok(Pending {
        scope: Scope {
            tenant: row.get(0),
            app: row.get(1),
        },
        redirect: row.get(2),
        state: row.get(3),
        expires: row.get(4),
        browser: row.get(5),
        csrf: row.get(6),
    })
}
fn bound(pending: &Pending, browser: &str, csrf: &str) -> Result<(), Failure> {
    if pending.browser.as_deref() != Some(digest(browser).as_str())
        || pending.csrf.as_deref() != Some(digest(csrf).as_str())
    {
        return Err("csrf_rejected");
    }
    Ok(())
}
pub(crate) fn safe_headers(response: &mut Response) {
    // Chromium sends Origin:null for native form POST under no-referrer.
    // Keep the password form same-origin while suppressing callback handoff referrers.
    let referrer_policy = if response.status() == StatusCode::SEE_OTHER {
        "no-referrer"
    } else {
        "same-origin"
    };
    let preserve_csp = response.extensions().get::<SafeHostedHtml>().is_some()
        && response.headers().contains_key("content-security-policy");
    for (name, value) in [
        ("cache-control", "no-store"),
        ("referrer-policy", referrer_policy),
        ("x-content-type-options", "nosniff"),
        (
            "content-security-policy",
            "default-src 'none'; form-action 'self'; frame-ancestors 'none'; base-uri 'none'",
        ),
    ] {
        if name == "content-security-policy" && preserve_csp {
            continue;
        }
        response
            .headers_mut()
            .insert(name, HeaderValue::from_static(value));
    }
}
fn html(status: StatusCode, message: &str, form: Option<&LoginForm>) -> Response {
    // Form context comes only from current DB/browser/CSRF validation.
    let policy = form.map(|form| {
        format!(
            "default-src 'none'; form-action 'self' {}; frame-ancestors 'none'; base-uri 'none'",
            form.callback_source
        )
    });
    let form = match form {
        Some(LoginForm { handle, csrf, .. }) => format!(
            "<form method=\"post\" action=\"/hosted/login\"><input type=\"hidden\" name=\"transaction\" value=\"{handle}\"><input type=\"hidden\" name=\"csrf_token\" value=\"{csrf}\"><p><label>Email <input name=\"email\" type=\"email\" autocomplete=\"username\" required></label></p><p><label>Password <input name=\"password\" type=\"password\" autocomplete=\"current-password\" required></label></p><button>Sign in</button></form>"
        ),
        _ => String::new(),
    };
    let body = format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><title>Kanche sign in</title></head><body><main><h1>Sign in with Kanche</h1><p>{message}</p>{form}</main></body></html>"
    );
    let mut response =
        (status, [("content-type", "text/html; charset=utf-8")], body).into_response();
    response.extensions_mut().insert(SafeHostedHtml);
    if let Some(policy) = policy {
        response.headers_mut().insert(
            "content-security-policy",
            HeaderValue::from_str(&policy).expect("validated callback source is CSP-safe"),
        );
    }
    safe_headers(&mut response);
    response
}
fn html_error(code: Failure, form: Option<&LoginForm>) -> Response {
    let (status, message) = match code {
        "authentication_failed" => (
            StatusCode::UNAUTHORIZED,
            "Sign-in could not be completed with those credentials.",
        ),
        "rate_limited" => (
            StatusCode::TOO_MANY_REQUESTS,
            "Too many attempts. Wait before retrying.",
        ),
        "csrf_rejected" => (
            StatusCode::FORBIDDEN,
            "The sign-in context is invalid. Restart sign-in from your application.",
        ),
        "temporarily_unavailable" => (
            StatusCode::SERVICE_UNAVAILABLE,
            "Temporarily unavailable. Retry or restart sign-in from your application.",
        ),
        _ => (
            StatusCode::BAD_REQUEST,
            "The sign-in context is invalid or expired. Restart sign-in from your application.",
        ),
    };
    let mut response = html(status, message, form);
    if code == "rate_limited" {
        response
            .headers_mut()
            .insert("retry-after", HeaderValue::from_static("900"));
    }
    response
}
fn strict_form(body: &[u8]) -> Result<Vec<(String, String)>, Failure> {
    fn decode(value: &[u8]) -> Result<String, Failure> {
        let mut decoded = Vec::with_capacity(value.len());
        let mut position = 0;
        while position < value.len() {
            match value[position] {
                b'+' => decoded.push(b' '),
                b'%' => {
                    let digits = value
                        .get(position + 1..position + 3)
                        .ok_or("invalid_request")?;
                    let high = (digits[0] as char).to_digit(16).ok_or("invalid_request")?;
                    let low = (digits[1] as char).to_digit(16).ok_or("invalid_request")?;
                    decoded.push((high * 16 + low) as u8);
                    position += 2;
                }
                byte => decoded.push(byte),
            }
            position += 1;
        }
        String::from_utf8(decoded).map_err(|_| "invalid_request")
    }
    if body.is_empty() {
        return Err("invalid_request");
    }
    body.split(|byte| *byte == b'&')
        .map(|pair| {
            let position = pair
                .iter()
                .position(|byte| *byte == b'=')
                .ok_or("invalid_request")?;
            Ok((decode(&pair[..position])?, decode(&pair[position + 1..])?))
        })
        .collect()
}
fn unique_pairs(pairs: Vec<(String, String)>, names: &[&str]) -> Result<Vec<String>, Failure> {
    if pairs.len() != names.len() {
        return Err("invalid_request");
    }
    names
        .iter()
        .map(|name| {
            let values: Vec<_> = pairs.iter().filter(|(key, _)| key == name).collect();
            if values.len() != 1 {
                return Err("invalid_request");
            }
            Ok(values[0].1.clone())
        })
        .collect()
}
fn browser_cookie(headers: &HeaderMap, development: bool) -> Result<Option<String>, Failure> {
    let name = if development {
        "kanche_hosted_dev_browser"
    } else {
        "__Host-kanche_hosted_browser"
    };
    let prefix = format!("{name}=");
    let mut found = None;
    for header in headers.get_all("cookie") {
        let value = header.to_str().map_err(|_| "invalid_handoff")?;
        for pair in value
            .split(';')
            .map(str::trim)
            .filter(|pair| pair.starts_with(&prefix))
        {
            let value = &pair[prefix.len()..];
            if found.is_some() || !hex_token(value) {
                return Err("invalid_handoff");
            }
            found = Some(value.to_owned());
        }
    }
    Ok(found)
}
fn cookie(value: &str, development: bool, remaining: i64) -> String {
    let name = if development {
        "kanche_hosted_dev_browser"
    } else {
        "__Host-kanche_hosted_browser"
    };
    format!(
        "{name}={value}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}{}",
        remaining.clamp(0, 300),
        if development { "" } else { "; Secure" }
    )
}
pub(crate) fn routes(store: Store) -> Router {
    Router::new()
        .route("/v1/hosted/transactions", post(create))
        .route("/v1/hosted/exchanges", post(exchange))
        .route("/hosted/login", get(login).post(password))
        .layer(DefaultBodyLimit::max(32768))
        .with_state(store)
}
async fn create(
    State(store): State<Store>,
    headers: HeaderMap,
    input: Result<Json<HostedTransaction>, axum::extract::rejection::JsonRejection>,
) -> Response {
    let input = match input {
        Ok(Json(input)) => input,
        Err(error) => return error.into_response(),
    };
    match store
        .create_hosted_transaction(crate::api::key(&headers), input)
        .await
    {
        Ok(value) => Json(value).into_response(),
        Err(code) => crate::api::error(code),
    }
}
async fn exchange(
    State(store): State<Store>,
    headers: HeaderMap,
    input: Result<Json<HostedExchange>, axum::extract::rejection::JsonRejection>,
) -> Response {
    let input = match input {
        Ok(Json(input)) => input,
        Err(error) => return error.into_response(),
    };
    match store
        .exchange_hosted(crate::api::key(&headers), input)
        .await
    {
        Ok(value) => Json(value).into_response(),
        Err(code) => crate::api::error(code),
    }
}
async fn login(
    State(store): State<Store>,
    headers: HeaderMap,
    query: Result<Query<Vec<(String, String)>>, QueryRejection>,
) -> Response {
    let pairs = match query {
        Ok(Query(pairs)) => pairs,
        Err(_) => return html_error("invalid_request", None),
    };
    let values = match unique_pairs(pairs, &["transaction"]) {
        Ok(values) => values,
        Err(code) => return html_error(code, None),
    };
    let config = store.hosted.as_ref().expect("hosted router configured");
    let browser = match browser_cookie(&headers, config.development) {
        Ok(browser) => browser,
        Err(code) => return html_error(code, None),
    };
    match store.hosted_page(&values[0], browser.as_deref()).await {
        Ok((browser, form)) => {
            let mut response = html(
                StatusCode::OK,
                "Enter your credentials to continue.",
                Some(&form),
            );
            // Reissue a bounded anonymous nonce. It neither identifies a user nor extends the transaction.
            response.headers_mut().append(
                "set-cookie",
                HeaderValue::from_str(&cookie(&browser, config.development, 300))
                    .expect("generated cookie is a header"),
            );
            response
        }
        Err(code) => html_error(code, None),
    }
}
async fn password(
    State(store): State<Store>,
    headers: HeaderMap,
    RawQuery(query): RawQuery,
    body: Result<Bytes, BytesRejection>,
) -> Response {
    if query.is_some() {
        return html_error("invalid_request", None);
    }
    let config = store.hosted.as_ref().expect("hosted router configured");
    let origins: Vec<_> = headers.get_all("origin").iter().collect();
    if origins.len() != 1 || origins[0].to_str().ok() != Some(config.origin.as_str()) {
        return html_error("csrf_rejected", None);
    }
    let browser = match browser_cookie(&headers, config.development) {
        Ok(Some(browser)) => browser,
        _ => return html_error("csrf_rejected", None),
    };
    let content_types: Vec<_> = headers.get_all("content-type").iter().collect();
    let supported = content_types.len() == 1
        && content_types[0].to_str().is_ok_and(|value| {
            let mut parts = value.split(';');
            parts.next().is_some_and(|mime| {
                mime.trim()
                    .eq_ignore_ascii_case("application/x-www-form-urlencoded")
            }) && parts.all(|parameter| parameter.trim().eq_ignore_ascii_case("charset=utf-8"))
        });
    if !supported {
        return StatusCode::UNSUPPORTED_MEDIA_TYPE.into_response();
    }
    let bytes = match body {
        Ok(bytes) => bytes,
        Err(error) => return error.into_response(),
    };
    let pairs = match strict_form(&bytes) {
        Ok(pairs) => pairs,
        Err(code) => return html_error(code, None),
    };
    let values = match unique_pairs(pairs, &["transaction", "csrf_token", "email", "password"]) {
        Ok(values) => values,
        Err(code) => return html_error(code, None),
    };
    match store
        .complete_hosted_password(
            &values[0],
            &values[1],
            &browser,
            SignIn {
                email: values[2].clone(),
                password: values[3].clone(),
            },
        )
        .await
    {
        Ok(location) => {
            let mut response = StatusCode::SEE_OTHER.into_response();
            response.headers_mut().insert(
                "location",
                HeaderValue::from_str(&location)
                    .expect("registered callback with generated query is a header"),
            );
            safe_headers(&mut response);
            response
        }
        Err(error) => html_error(error.code, error.form.as_ref()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn callback_csp_source_encodes_directive_delimiters() {
        let form = LoginForm::new(&secret(), &secret(), "https://app.example/a;'callback").unwrap();
        assert!(form.callback_source == "https://app.example/a%3B%27callback");
        assert!(!form.callback_source.contains(';') && !form.callback_source.contains('\''));
        let response = html_error("authentication_failed", Some(&form));
        let policy = response.headers()["content-security-policy"]
            .to_str()
            .unwrap();
        assert!(policy.contains("form-action 'self' https://app.example/a%3B%27callback;"));
        assert!(!policy.contains('*'));
    }
    #[test]
    fn password_form_decoding_preserves_utf8_or_rejects_it() {
        let decoded = strict_form(b"password=a%2Bb+c%3D%C3%A9").unwrap();
        assert!(decoded[0].1 == "a+b c=é");
        for invalid in [
            b"password=%FF".as_slice(),
            b"password=%C0%80",
            b"password=%",
            b"password=%0",
            b"password=%GG",
            b"",
            b"password=x&",
        ] {
            assert!(strict_form(invalid).is_err());
        }
    }
    #[test]
    fn callback_and_origin_validation_are_strict() {
        for uri in [
            "https://app.example/callback",
            "https://app.example/api/kanche/callback",
        ] {
            assert!(canonical_url(uri, false, true).is_ok());
        }
        for uri in [
            "https://app.example/",
            "https://app.example/callback?x=1",
            "https://app.example/callback#fragment",
            "https://user:password@app.example/callback",
            "https://APP.example/callback",
            "https://app.example:443/callback",
            "https://app.example/a/../callback",
            "https://app.example/%63allback",
            "https://app.example/\\callback",
            "https://*.example/callback",
            "http://app.example/callback",
            "https://app.example/callback\n",
        ] {
            assert!(
                canonical_url(uri, false, true).is_err(),
                "invalid callback accepted"
            );
        }
        assert!(canonical_url("http://localhost:3010/api/kanche/callback", true, true).is_ok());
        assert!(canonical_url("http://127.0.0.1:18080", true, false).is_ok());
        assert!(canonical_url("http://[::1]:18080", true, false).is_ok());
        assert!(canonical_url("http://[::1]:3010/api/kanche/callback", true, true).is_ok());
        assert!(canonical_url("http://[2001:db8::1]:18080", true, false).is_err());
        assert!(canonical_url("http://127.0.0.1:18080/", true, false).is_err());
        assert!(canonical_url("http://app.example/callback", true, true).is_err());
    }
}
