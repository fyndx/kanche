// SPDX-License-Identifier: MPL-2.0
//! Durable, online-only app identity boundary. No organization policy is implied.
use argon2::{
    Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString,
};
use rand::{RngCore, rngs::OsRng};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{sync::Arc, time::Duration};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

mod hosted;
pub use hosted::{HostedExchange, HostedRegistration, HostedTransaction};
pub(crate) use hosted::{SafeHostedHtml, routes as hosted_routes, safe_headers as hosted_headers};
use tokio_postgres::{Client, NoTls, Transaction, config::Host};

fn db_error(error: tokio_postgres::Error) -> Failure {
    tracing::warn!(error_kind=?error.code(),"Database operation unavailable");
    "temporarily_unavailable"
}

pub type Failure = &'static str;
pub fn digest(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))
}
pub fn secret() -> String {
    let mut bytes = [0; 32];
    OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}
fn argon() -> Argon2<'static> {
    Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        Params::new(19456, 2, 1, None).expect("fixed parameters"),
    )
}
pub fn password_hash(password: &str) -> Result<String, Failure> {
    argon()
        .hash_password(password.as_bytes(), &SaltString::generate(&mut OsRng))
        .map(|x| x.to_string())
        .map_err(|_| "temporarily_unavailable")
}
pub fn email(value: &str) -> Result<String, Failure> {
    let value = value.trim().to_ascii_lowercase();
    let parts: Vec<_> = value.split('@').collect();
    if !value.is_ascii()
        || value.len() > 254
        || parts.len() != 2
        || parts.iter().any(|x| x.is_empty())
        || value
            .bytes()
            .any(|b| b.is_ascii_whitespace() || b.is_ascii_control())
    {
        return Err("invalid_request");
    }
    Ok(value)
}
#[derive(Clone)]
pub struct Store {
    config: tokio_postgres::Config,
    dummy: Arc<String>,
    admission: Arc<Semaphore>,
    connections: Arc<Semaphore>,
    hosted_pages: Arc<Semaphore>,
    hosted: Option<Arc<hosted::HostedConfig>>,
}
impl Store {
    pub fn new(url: &str) -> Result<Self, Failure> {
        let mut config: tokio_postgres::Config =
            url.parse().map_err(|_| "Invalid KANCHE_DATABASE_URL")?;
        // The first slice deliberately cannot connect to a remote plaintext database.
        if config.get_hosts().is_empty() || config.get_hosts().iter().any(|h| !matches!(h,Host::Tcp(s) if s.parse::<std::net::IpAddr>().is_ok_and(|ip|ip.is_loopback()))) { return Err("Database must use an explicit loopback IP; remote TLS profile is not delivered") }
        config.connect_timeout(Duration::from_secs(2));
        Ok(Self {
            config,
            dummy: Arc::new(password_hash(&secret())?),
            admission: Arc::new(Semaphore::new(8)),
            connections: Arc::new(Semaphore::new(16)),
            hosted_pages: Arc::new(Semaphore::new(4)),
            hosted: None,
        })
    }
    pub async fn connect(&self) -> Result<Client, Failure> {
        let permit = self
            .connections
            .clone()
            .try_acquire_owned()
            .map_err(|_| "temporarily_unavailable")?;
        let (client, connection) =
            tokio::time::timeout(Duration::from_secs(2), self.config.connect(NoTls))
                .await
                .map_err(|_| "temporarily_unavailable")?
                .map_err(db_error)?;
        tokio::spawn(async move {
            let _permit = permit;
            let _ = connection.await;
        });
        client
            .batch_execute(
                "
            SET statement_timeout='3s'; SET lock_timeout='2s'; SET
            idle_in_transaction_session_timeout='5s'
        ",
            )
            .await
            .map_err(db_error)?;
        Ok(client)
    }
    pub async fn begin(client: &mut Client) -> Result<Transaction<'_>, Failure> {
        let tx = client.transaction().await.map_err(db_error)?;
        tx.batch_execute("SELECT pg_advisory_xact_lock(7248162)")
            .await
            .map_err(db_error)?;
        Ok(tx)
    }
    pub async fn migrate(&self) -> Result<(), Failure> {
        let mut c = self.connect().await?;
        let tx = Self::begin(&mut c).await?;
        tx.batch_execute(
            "
            CREATE TABLE IF NOT EXISTS kanche_migrations(version integer PRIMARY KEY,
            digest text NOT NULL)
        ",
        )
        .await
        .map_err(|_| "migration_failed")?;
        for (version, sql) in [
            (1, include_str!("../migrations/0001_app_login.sql")),
            (2, include_str!("../migrations/0002_hosted_login.sql")),
        ] {
            let checksum = digest(sql);
            match tx
                .query_opt(
                    "SELECT digest FROM kanche_migrations WHERE version=$1",
                    &[&version],
                )
                .await
                .map_err(|_| "migration_failed")?
            {
                Some(row) if row.get::<_, String>(0) != checksum => {
                    return Err("migration_checksum_mismatch");
                }
                Some(_) => {}
                None => {
                    tx.batch_execute(sql)
                        .await
                        .map_err(|_| "migration_failed")?;
                    tx.execute(
                        "INSERT INTO kanche_migrations VALUES($1,$2)",
                        &[&version, &checksum],
                    )
                    .await
                    .map_err(|_| "migration_failed")?;
                }
            }
        }
        tx.commit().await.map_err(|_| "migration_failed")
    }
    pub async fn scope(tx: &Transaction<'_>, app_secret: &str) -> Result<Scope, Failure> {
        if app_secret.len() != 64 {
            return Err("invalid_application_credentials");
        }
        let row = tx
            .query_opt(
                "
            SELECT a.tenant_id,a.id FROM application_keys k JOIN applications a ON
            a.id=k.app_id JOIN tenants t ON t.id=a.tenant_id WHERE k.digest=$1 AND
            a.active AND t.active
        ",
                &[&digest(app_secret)],
            )
            .await
            .map_err(db_error)?
            .ok_or("invalid_application_credentials")?;
        Ok(Scope {
            tenant: row.get(0),
            app: row.get(1),
        })
    }
    pub async fn application(&self, key: &str) -> Result<serde_json::Value, Failure> {
        let mut c = self.connect().await?;
        let tx = Self::begin(&mut c).await?;
        let s = Self::scope(&tx, key).await?;
        let mut features = vec!["password_login", "opaque_sessions"];
        if self.hosted.is_some() {
            features.push("hosted_login");
        }
        Ok(
            serde_json::json!({"tenant_id":s.tenant,"application_id":s.app,"api_version":"v1","features":features}),
        )
    }
    pub async fn sign_in(&self, key: &str, input: SignIn) -> Result<serde_json::Value, Failure> {
        let permit = self
            .admission
            .clone()
            .try_acquire_owned()
            .map_err(|_| "temporarily_unavailable")?;
        validate_password_input(&input)?;
        let mut client = self.connect().await?;
        let tx = Self::begin(&mut client).await?;
        let scope = Self::scope(&tx, key).await?;
        let prepared = match self.prepare_password(&tx, &scope, input).await {
            Ok(prepared) => prepared,
            Err(error @ "rate_limited") => {
                tx.commit().await.map_err(db_error)?;
                return Err(error);
            }
            Err(error) => return Err(error),
        };
        // Admission is durable even if the caller disconnects during password work.
        tx.commit().await.map_err(db_error)?;
        let proof = prepared.verify(permit).await?;
        let tx = Self::begin(&mut client).await?;
        let final_scope = Self::scope(&tx, key).await?;
        if final_scope.tenant != scope.tenant || final_scope.app != scope.app {
            return Err("invalid_application_credentials");
        }
        if !valid_proof(&tx, &final_scope, &proof).await? {
            return Err("authentication_failed");
        }
        let result = issue(&tx, &final_scope, &proof.user).await?;
        tx.commit().await.map_err(db_error)?;
        Ok(result)
    }
    async fn prepare_password(
        &self,
        tx: &Transaction<'_>,
        scope: &Scope,
        input: SignIn,
    ) -> Result<PreparedPassword, Failure> {
        let address = validate_password_input(&input)?;
        let timestamp = now(tx).await?;
        for (bucket, window, limit) in [
            (format!("app:{}", scope.app), 60, 30),
            (
                format!("account:{}:{}", scope.app, digest(&address)),
                900,
                5,
            ),
        ] {
            rate(tx, &bucket, timestamp, window, limit).await?;
        }
        let row = tx
            .query_opt(
                "SELECT id,password_hash FROM users WHERE tenant_id=$1 AND email=$2",
                &[&scope.tenant, &address],
            )
            .await
            .map_err(db_error)?;
        Ok(PreparedPassword {
            user: row.as_ref().map(|r| r.get(0)),
            hash: row
                .as_ref()
                .map_or_else(|| self.dummy.as_ref().clone(), |r| r.get(1)),
            password: input.password,
        })
    }
    pub async fn session(
        &self,
        key: &str,
        credential: &str,
        operation: &str,
    ) -> Result<serde_json::Value, Failure> {
        let mut c = self.connect().await?;
        let tx = Self::begin(&mut c).await?;
        let s = Self::scope(&tx, key).await?;
        if operation == "revoke" {
            tx.execute(
                "UPDATE sessions SET revoked=true WHERE digest=$1 AND app_id=$2 AND tenant_id=$3",
                &[&digest(credential), &s.app, &s.tenant],
            )
            .await
            .map_err(db_error)?;
            tx.commit().await.map_err(db_error)?;
            return Ok(serde_json::Value::Null);
        }
        if credential.len() != 64 {
            return Err("unauthenticated");
        }
        let now = now(&tx).await?;
        let row = tx
            .query_opt(
                "
            SELECT user_id,idle_expires_at,absolute_expires_at FROM sessions WHERE
            digest=$1 AND tenant_id=$2 AND app_id=$3 AND NOT revoked AND
            idle_expires_at>$4 AND absolute_expires_at>$4
        ",
                &[&digest(credential), &s.tenant, &s.app, &now],
            )
            .await
            .map_err(db_error)?
            .ok_or("unauthenticated")?;
        let user: String = row.get(0);
        if !eligible(&tx, &s, &user).await? {
            return Err("unauthenticated");
        }
        let mut idle: i64 = row.get(1);
        let absolute: i64 = row.get(2);
        if operation == "renew" {
            idle = idle.max((now + 86400).min(absolute));
            tx.execute(
                "UPDATE sessions SET idle_expires_at=GREATEST(idle_expires_at,$1) WHERE digest=$2",
                &[&idle, &digest(credential)],
            )
            .await
            .map_err(db_error)?;
        }
        tx.commit().await.map_err(db_error)?;
        Ok(view(&s, &user, idle, absolute))
    }
}
fn validate_password_input(input: &SignIn) -> Result<String, Failure> {
    let address = email(&input.email)?;
    if input.password.is_empty() || input.password.len() > 128 {
        return Err("invalid_request");
    }
    Ok(address)
}
struct PreparedPassword {
    user: Option<String>,
    hash: String,
    password: String,
}
struct PasswordProof {
    user: String,
    hash_digest: String,
}
impl PreparedPassword {
    async fn verify(self, permit: OwnedSemaphorePermit) -> Result<PasswordProof, Failure> {
        let expected = digest(&self.hash);
        let valid = password_work(permit, move || {
            PasswordHash::new(&self.hash).is_ok_and(|h| {
                argon()
                    .verify_password(self.password.as_bytes(), &h)
                    .is_ok()
            })
        })
        .await?;
        match (valid, self.user) {
            (true, Some(user)) => Ok(PasswordProof {
                user,
                hash_digest: expected,
            }),
            _ => Err("authentication_failed"),
        }
    }
}
async fn valid_proof(
    tx: &Transaction<'_>,
    scope: &Scope,
    proof: &PasswordProof,
) -> Result<bool, Failure> {
    let row = tx
        .query_opt(
            "SELECT password_hash FROM users WHERE tenant_id=$1 AND id=$2 AND active AND verified",
            &[&scope.tenant, &proof.user],
        )
        .await
        .map_err(db_error)?;
    let Some(row) = row else { return Ok(false) };
    Ok(digest(&row.get::<_, String>(0)) == proof.hash_digest
        && eligible(tx, scope, &proof.user).await?)
}
async fn password_work<F>(permit: OwnedSemaphorePermit, work: F) -> Result<bool, Failure>
where
    F: FnOnce() -> bool + Send + 'static,
{
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        work()
    })
    .await
    .map_err(|_| "temporarily_unavailable")
}
async fn rate(
    tx: &Transaction<'_>,
    bucket: &str,
    timestamp: i64,
    window: i64,
    limit: i32,
) -> Result<(), Failure> {
    let attempts:i32=tx.query_one(
        "INSERT INTO abuse_buckets VALUES($1,$2,1) ON CONFLICT(bucket) DO UPDATE SET
         attempts=CASE WHEN abuse_buckets.expires_at <= $3 THEN 1 ELSE abuse_buckets.attempts+1 END,
         expires_at=CASE WHEN abuse_buckets.expires_at <= $3 THEN $2 ELSE abuse_buckets.expires_at END
         RETURNING attempts",&[&bucket,&(timestamp+window),&timestamp]).await.map_err(db_error)?.get(0);
    if attempts > limit {
        Err("rate_limited")
    } else {
        Ok(())
    }
}
async fn issue(
    tx: &Transaction<'_>,
    scope: &Scope,
    user: &str,
) -> Result<serde_json::Value, Failure> {
    let timestamp = now(tx).await?;
    let credential = secret();
    let idle = timestamp + 86400;
    let absolute = timestamp + 604800;
    tx.execute(
        "INSERT INTO sessions VALUES($1,$2,$3,$4,$5,$6,false)",
        &[
            &digest(&credential),
            &scope.tenant,
            &scope.app,
            &user,
            &idle,
            &absolute,
        ],
    )
    .await
    .map_err(db_error)?;
    let mut result = view(scope, user, idle, absolute);
    result["session"]["credential"] = credential.into();
    Ok(result)
}
pub struct Scope {
    pub tenant: String,
    pub app: String,
}
async fn eligible(tx: &Transaction<'_>, s: &Scope, user: &str) -> Result<bool, Failure> {
    Ok(tx
        .query_one(
            "
            SELECT EXISTS(SELECT 1 FROM users u JOIN enrollments e ON e.user_id=u.id AND
            e.tenant_id=u.tenant_id WHERE u.id=$1 AND u.tenant_id=$2 AND u.active AND
            u.verified AND e.app_id=$3 AND e.active)
        ",
            &[&user, &s.tenant, &s.app],
        )
        .await
        .map_err(db_error)?
        .get(0))
}
async fn now(tx: &Transaction<'_>) -> Result<i64, Failure> {
    Ok(tx
        .query_one(
            "SELECT floor(extract(epoch FROM clock_timestamp()))::bigint",
            &[],
        )
        .await
        .map_err(db_error)?
        .get(0))
}
fn view(s: &Scope, user: &str, idle: i64, absolute: i64) -> serde_json::Value {
    serde_json::json!({"user":{"id":user,"tenant_id":s.tenant,"application_id":s.app},"session":{"format":"opaque","idle_expires_at":idle,"absolute_expires_at":absolute}})
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SignIn {
    pub email: String,
    pub password: String,
}

/// Operator-only provisioning. Input is read from stdin, never command-line secrets.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provision {
    pub tenant_id: String,
    pub application_id: String,
    pub user_id: String,
    pub email: String,
    pub password: String,
    pub application_secret: String,
}
impl Store {
    pub async fn provision(&self, p: Provision) -> Result<(), Failure> {
        if [&p.tenant_id, &p.application_id, &p.user_id]
            .iter()
            .any(|v| {
                v.is_empty()
                    || v.len() > 128
                    || !v
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
            })
            || !(15..=128).contains(&p.password.len())
            || p.application_secret.len() != 64
            || !p.application_secret.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err("invalid_provisioning_input");
        }
        let address = email(&p.email)?;
        let hash = password_hash(&p.password)?;
        let mut c = self.connect().await?;
        let tx = Self::begin(&mut c).await?;
        tx.execute(
            "INSERT INTO tenants VALUES($1,true) ON CONFLICT DO NOTHING",
            &[&p.tenant_id],
        )
        .await
        .map_err(|_| "provisioning_conflict")?;
        tx.execute(
            "INSERT INTO applications VALUES($1,$2,true) ON CONFLICT DO NOTHING",
            &[&p.application_id, &p.tenant_id],
        )
        .await
        .map_err(|_| "provisioning_conflict")?;
        if !tx
            .query_one(
                "SELECT tenant_id=$2 FROM applications WHERE id=$1",
                &[&p.application_id, &p.tenant_id],
            )
            .await
            .map_err(|_| "provisioning_conflict")?
            .get::<_, bool>(0)
        {
            return Err("provisioning_conflict");
        }
        tx.execute(
            "INSERT INTO application_keys VALUES($1,$2) ON CONFLICT DO NOTHING",
            &[&digest(&p.application_secret), &p.application_id],
        )
        .await
        .map_err(|_| "provisioning_conflict")?;
        if !tx
            .query_one(
                "SELECT app_id=$2 FROM application_keys WHERE digest=$1",
                &[&digest(&p.application_secret), &p.application_id],
            )
            .await
            .map_err(|_| "provisioning_conflict")?
            .get::<_, bool>(0)
        {
            return Err("provisioning_conflict");
        }
        let inserted = tx
            .execute(
                "INSERT INTO users VALUES($1,$2,$3,$4,true,true) ON CONFLICT DO NOTHING",
                &[&p.user_id, &p.tenant_id, &address, &hash],
            )
            .await
            .map_err(|_| "provisioning_conflict")?;
        if inserted == 0 {
            let row = tx
                .query_opt(
                    "SELECT tenant_id,email,password_hash FROM users WHERE id=$1",
                    &[&p.user_id],
                )
                .await
                .map_err(|_| "provisioning_conflict")?
                .ok_or("provisioning_conflict")?;
            if row.get::<_, String>(0) != p.tenant_id
                || row.get::<_, String>(1) != address
                || !PasswordHash::new(&row.get::<_, String>(2))
                    .is_ok_and(|h| argon().verify_password(p.password.as_bytes(), &h).is_ok())
            {
                return Err("provisioning_conflict");
            }
        }
        tx.execute(
            "INSERT INTO enrollments VALUES($1,$2,$3,true) ON CONFLICT DO NOTHING",
            &[&p.tenant_id, &p.application_id, &p.user_id],
        )
        .await
        .map_err(|_| "provisioning_conflict")?;
        tx.commit().await.map_err(|_| "provisioning_conflict")
    }
    pub async fn maintenance(&self, operation: &str, app: &str, user: &str) -> Result<(), Failure> {
        let mut c = self.connect().await?;
        let tx = Self::begin(&mut c).await?;
        let target_exists = match operation {
            "remove-enrollment" => {
                tx.query_opt(
                    "SELECT 1 FROM enrollments WHERE app_id=$1 AND user_id=$2",
                    &[&app, &user],
                )
                .await
            }
            "disable-user" => {
                tx.query_opt("SELECT 1 FROM users WHERE id=$1", &[&user])
                    .await
            }
            "disable-app" | "revoke-app-sessions" => {
                tx.query_opt("SELECT 1 FROM applications WHERE id=$1", &[&app])
                    .await
            }
            "remove-key" => {
                tx.query_opt("SELECT 1 FROM application_keys WHERE digest=$1", &[&app])
                    .await
            }
            _ => Ok(Some(
                tx.query_one("SELECT 1", &[])
                    .await
                    .map_err(|_| "maintenance_failed")?,
            )),
        }
        .map_err(|_| "maintenance_failed")?;
        if target_exists.is_none() {
            return Err("maintenance_target_not_found");
        }
        match operation {
            "cleanup" => {
                let now = now(&tx).await?;
                tx.execute("DELETE FROM sessions WHERE revoked OR idle_expires_at<=$1 OR absolute_expires_at<=$1",&[&now]).await.map_err(|_|"maintenance_failed")?;
                tx.execute("DELETE FROM hosted_transactions WHERE expires_at<=$1 OR consumed OR invalidated OR code_expires_at<=$1", &[&now]).await.map_err(|_|"maintenance_failed")?;
                tx.execute("DELETE FROM abuse_buckets WHERE expires_at<=$1", &[&now])
                    .await
                    .map_err(|_| "maintenance_failed")?;
            }
            "revoke-all" => {
                tx.execute("UPDATE hosted_transactions SET invalidated=true", &[])
                    .await
                    .map_err(|_| "maintenance_failed")?;
                tx.execute("UPDATE sessions SET revoked=true", &[])
                    .await
                    .map_err(|_| "maintenance_failed")?;
            }
            "remove-enrollment" => {
                tx.execute("UPDATE hosted_transactions SET invalidated=true WHERE app_id=$1 AND user_id=$2", &[&app,&user]).await.map_err(|_|"maintenance_failed")?;
                tx.execute(
                    "UPDATE enrollments SET active=false WHERE app_id=$1 AND user_id=$2",
                    &[&app, &user],
                )
                .await
                .map_err(|_| "maintenance_failed")?;
                tx.execute(
                    "UPDATE sessions SET revoked=true WHERE app_id=$1 AND user_id=$2",
                    &[&app, &user],
                )
                .await
                .map_err(|_| "maintenance_failed")?;
            }
            "disable-user" => {
                tx.execute(
                    "UPDATE hosted_transactions SET invalidated=true WHERE user_id=$1",
                    &[&user],
                )
                .await
                .map_err(|_| "maintenance_failed")?;
                tx.execute("UPDATE users SET active=false WHERE id=$1", &[&user])
                    .await
                    .map_err(|_| "maintenance_failed")?;
                tx.execute(
                    "UPDATE sessions SET revoked=true WHERE user_id=$1",
                    &[&user],
                )
                .await
                .map_err(|_| "maintenance_failed")?;
            }
            "disable-app" | "revoke-app-sessions" => {
                tx.execute(
                    "UPDATE hosted_transactions SET invalidated=true WHERE app_id=$1",
                    &[&app],
                )
                .await
                .map_err(|_| "maintenance_failed")?;
                if operation == "disable-app" {
                    tx.execute("UPDATE applications SET active=false WHERE id=$1", &[&app])
                        .await
                        .map_err(|_| "maintenance_failed")?;
                }
                tx.execute("UPDATE sessions SET revoked=true WHERE app_id=$1", &[&app])
                    .await
                    .map_err(|_| "maintenance_failed")?;
            }
            "remove-key" => {
                if app.len() != 64 || !app.bytes().all(|b| b.is_ascii_hexdigit()) {
                    return Err("invalid_key_digest");
                }
                tx.execute("DELETE FROM application_keys WHERE digest=$1", &[&app])
                    .await
                    .map_err(|_| "maintenance_failed")?;
            }
            _ => return Err("unsupported_operator_command"),
        }
        tx.commit().await.map_err(|_| "maintenance_failed")
    }
}

#[cfg(test)]
mod cancellation_tests {
    use super::*;
    #[tokio::test]
    async fn database_admission_rejects_before_connecting() {
        let store = Store::new("postgres://postgres@127.0.0.1:1/postgres").unwrap();
        let permit = store
            .connections
            .clone()
            .try_acquire_many_owned(16)
            .unwrap();
        assert!(matches!(
            store.connect().await,
            Err("temporarily_unavailable")
        ));
        drop(permit);
        assert_eq!(store.connections.available_permits(), 16);
    }
    #[tokio::test]
    async fn cancelled_password_request_keeps_hash_admission_until_worker_finishes() {
        let admission = Arc::new(Semaphore::new(1));
        let permit = admission.clone().try_acquire_owned().unwrap();
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let task = tokio::spawn(password_work(permit, move || {
            let _ = started_tx.send(());
            release_rx.recv().unwrap();
            true
        }));
        started_rx.await.unwrap();
        task.abort();
        let _ = task.await;
        assert!(admission.clone().try_acquire_owned().is_err());
        release_tx.send(()).unwrap();
        assert!(
            tokio::time::timeout(Duration::from_secs(2), admission.acquire())
                .await
                .is_ok()
        );
    }
}
