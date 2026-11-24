# App-owned login integration (initial slice)

The Rust service, `@kanche/server` and `@kanche/nextjs` packages are unpublished.
The reviewed [contract/lifecycle](../architecture/0003-app-owned-login.md) and
[OpenAPI 3.1 specification](app-owned.openapi.json) define the first profile.
This slice accepts provisioned verified synthetic accounts. Signup, invitations,
email verification, password recovery, sign out everywhere, JWT and organizations
are not delivered. The required [hosted profile](hosted-integration.md) complements
this first app-owned profile using the same online sessions.

## Fresh authorized checkout

Use the pinned prerequisites in the [README](../../README.md), plus Docker with
permission to start a loopback PostgreSQL container. No provider account is needed.

```sh
make setup
make check
make security
```

`make check` includes a disposable PostgreSQL migration/isolation/lifecycle test,
TypeScript type checks, SDK/adapter security tests and OpenAPI validation, as well
as the original process and repository checks. The database uses a pinned image
digest and a random loopback port, trusts local test connections, and is removed
on exit. This is a disposable test setup, never deployment configuration.

The repository checks use synthetic identities and ephemeral test keys. See
[verification coverage](../verification/app-owned-login.md) for repeatable checks
and operational limits.

## Operator provisioning / migrations

Service startup never silently migrates. Supply `KANCHE_DATABASE_URL` in process
environment, then run `cargo run --locked --bin kanche-admin -- migrate` before
starting `cargo run --locked --bin kanche`. Without database config the original
health-only router remains available. Invalid config fails startup; a database
outage returns 503 for protected calls. The first database connector deliberately
accepts explicit loopback IPs only: remote TLS/pooling is a separate delivery.

`kanche-admin provision-synthetic` accepts bounded JSON on stdin with tenant_id,
application_id, user_id, email, password and application_secret. Do not pass raw
secrets as shell arguments, commit provisioning JSON or use this synthetic
verified-account shortcut for public onboarding. The private harness is a complete
provisioning example with ephemeral values. IDs must be ASCII alphanumeric, `_`
or `-`, 1..128 bytes. Application secret is 32 random bytes encoded as 64 hex digits.
Provisioning uses a salted Argon2id hash; 15..128-byte passwords are never changed
or truncated. ASCII emails are trimmed/lowercased, at most 254 bytes with exactly
one `@` and nonempty parts. This is not full email syntax or ownership validation.
One tenant uses the built-in password identity store; no configurable connection
or organization model is implied. Repeating identical provisioning is safe;
conflicting tenant/app/user identity or password fails without reassignment.
Existing inactive accounts/enrollments are not silently reenabled.

Operator-only maintenance commands (same database authority as migrations):

- `cleanup`: remove expired/revoked sessions and expired rate buckets.
- `remove-enrollment APP USER`: deactivate that enrollment and revoke its sessions.
- `disable-user UNUSED USER`: disable a tenant user and revoke all their app sessions.
- `disable-app APP`: disable the app and revoke all its sessions.
- `revoke-app-sessions APP`: revoke that app’s sessions and invalidate hosted
  transactions while leaving the app active. Use after removing a compromised key.
- `remove-key SHA256_DIGEST`: remove a stored application key digest; supply the
  digest, never a raw credential. Provisioning the same app/user with a new random
  application secret adds a rollout key. Remove old digests after rollout.
- `revoke-all`: recovery operation to revoke every session in this database.
  Run after restoring a backup before accepting protected traffic.

These commands have no HTTP surface. App credentials cannot provision or
administer users. No production database or secrets are provisioned by this work.
Migrations are transactional, serialized across instances and checksum checked.
Rollback of this initial schema means stopping auth traffic and restoring the
pre-migration database; no destructive automated down migration is provided.
Backups and full operational restore drills are not established by local tests.

## Server SDK and Next.js boundary

Install from authorized local source/workspace until a separate package release.
Import `Kanche` from `@kanche/server` on the server; configure baseUrl,
applicationSecret, tenantId and applicationId. Explicit `development:true` allows
loopback HTTP. `signIn(email,password)` returns a nonserializable/redacted `Grant`;
`serverCredential()` is deliberately explicit. `verify`, `renew` and `revoke`
accept that credential. Don't log it. SDKs never retry passwords automatically,
follow redirects, or positively cache verification. Errors carry a stable code
and status, with no raw transport/configuration details. Both request deadline
and received response size are bounded. Unknown success formats/scopes fail closed.

Import `KancheAdapter` from `@kanche/nextjs/server`; its `server-only` guard rejects
client imports. Configure the SDK fields plus canonical `origin`, `cookieKeys`
(key ID to 32-byte hex key) and `currentKey`. `development:true` is restricted to
loopback origins. The private demo shows full App Router integration.

`handle(request, operation)` supports GET `csrf` and `session`, POST `sign-in`,
`renew`, `sign-out`. Browser POSTs use same-origin JSON and `x-csrf-token` from the
short-lived CSRF transaction. Fetch a fresh transaction after login/key changes.
Mutations outside this set fail. Redirect destination is fixed `/protected` in
the demo. Responses never include grants/app secrets. `read(cookieHeader)` is a
read-only server verification suitable for Server Components or route handlers:
unauthenticated state is separate from an unavailable exception. The demo's
protected route returns actual HTTP 503 during an outage.

The adapter uses AES-256-GCM host-only HttpOnly cookies (deployment `__Host-`
prefix + Secure; loopback dev has a separate name). Cookie lifetime is bounded by
absolute expiry; PostgreSQL enforces idle expiry. GET verification does not renew.
Explicit foreground activity calls the CSRF-protected renewal route. Stable bearer
renewal needs no Set-Cookie rewrite, so a lost renewal response leaves the existing
cookie usable. Ordinary sign-out confirms upstream revocation before clearing the
cookie; if unavailable, return 503 and keep it for retry. Other apps/devices stay
active. Unexpected session format never authenticates a browser.

For cookie key rotation deploy all readers with old/new keys, switch currentKey,
retain old readers for at most the seven-day session lifetime, then remove old.
Emergency reader-key removal forces local login; also revoke upstream sessions
if compromised. Application secrets and cookie encryption keys are separate.

## Operational limits

Only loopback deployment is exercised. HTTPS ingress, remote database TLS,
capacity tuning, HA, real mail, key management, browser matrix and backup drills
remain undelivered. A global database transaction lock intentionally serializes
authority decisions across instances: scale and contention require measurement.
Abuse limits are durable per app/minute and app/account/15 minutes, not an edge
IP defense. Invalid app credentials and connect attempts still need ingress
admission limits. A stolen opaque credential remains replayable when paired with
valid app authority until expiry/revocation. No zero-latency revocation promise:
next ordered authoritative checks reject; already-authorized app work may finish.
Lost sign-in responses or browser cookie rejection can leave an orphan session;
bounded expiry and cleanup apply. No browser persistence acknowledgment or
exactly-once login is claimed. Passing local checks does not establish readiness.

Database admission is bounded per Store (16 concurrent connections); anonymous
hosted page reads have an additional four-request admission limit. Saturation
returns `temporarily_unavailable`. These are process-local limits; deployment
replica counts and database capacity still require load testing. The shared
authority lock remains a throughput limit.

Targeted operator commands fail with `maintenance_target_not_found` when the
target does not exist. Missing or empty CLI arguments fail before execution.
