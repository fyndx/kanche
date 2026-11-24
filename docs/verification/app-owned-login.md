# App-owned login verification

## Repeatable checks

Run `make check` with the pinned toolchains and Docker. It runs formatting,
Clippy, lint and documentation checks, Rust tests, process smoke checks,
TypeScript checks, SDK/adapter tests, OpenAPI validation, PostgreSQL integration
scenarios, and Rust documentation. Plain `cargo test` skips the database scenarios;
`make check` starts disposable PostgreSQL and runs them explicitly.

The database scenario covers migrations and provisioning conflicts, tenant/app
isolation, failed and ineligible login, expiry boundaries, renewal concurrency,
revocation, durable throttling, lock contention and recovery, and cleanup.
SDK/adapter tests cover response validation, redaction, CSRF/origin checks,
encrypted cookies, tampering, key rollover, and outage handling.

`make security` checks secrets, dependency advisories, and licenses. Passing checks
provide bounded evidence for the tested revision, not production readiness.

## Coverage limits

The database suite combines multiple invariants into one scenario; its test count
is not independent exhaustive coverage. The lock-timeout case intentionally logs
SQLSTATE 55P03 without parameters or credentials.

Deployment TLS, remote database TLS, edge abuse tuning, capacity/HA, managed keys,
backup/restore drills, and a full browser matrix remain unverified by these checks.
Global transaction locking limits throughput. Valid bearer replay remains possible
until expiry or revocation. In-flight application work may finish after revocation.
Cookies require synchronized clocks. Lost login responses or rejected cookies can
leave bounded orphan sessions; exactly-once issuance is not claimed.

See the [integration guide](../contracts/app-owned-integration.md) and
[hosted verification](hosted-login.md) for related behavior and limits.
