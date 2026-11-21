# KN-2 / KN-6: first app-owned login design gate

Engineering decision, 21 November 2026, within the user's approved baseline.
Reviewed AGENTS.md, roadmap, current Confluence 99090434 / 99221506 / 98926594
and KN-2/KN-6/KN-11 decisions. Older contradictory proposals are superseded.
This completes the design prerequisites for the bounded app-owned slice below;
it is not evidence of implemented behavior or production readiness.

## Named delivery slices and boundaries

1. KN-3 foundation: PostgreSQL versioned migration, operator-only provisioning
   of tenant, app, verified synthetic tenant user, separate direct app enrollment.
   No public provisioning or administrator bearer shared with app requests.
2. KN-4 prerequisite subset / KN-5: provisioned password identity and completed
   password login. Public signup, email delivery/verification, invitations and
   recovery remain undelivered; never advertise them.
3. KN-7/8/9: opaque issuance, inspection, explicit idle renewal, current-session
   revocation; TypeScript server SDK, Next.js adapter, private demo and HTTP client.

KN-11 stays open: no organization tables, claims or endpoints. Direct enrollment
is separate from tenant identity and isolated behind an eligibility function;
future organization grants require reviewed policy and migration, not a promise
that direct enrollment is the only future access source. Hosted login is required
next: exact registered callback, expiring state and PKCE-bound single-use server
handoff, same app eligibility and sessions. Hosted transaction implementation
requires its detailed contract. Future JWT purpose/timing and conformance remain
open; credential format is explicit and identity is independent.

## Concrete API / SDK / demo contract

Version /v1. Confidential application authenticates with Authorization: Bearer
application secret. Session operations also require Kanche-Session. Tenant/app
scope comes only from authenticated app authority. JSON request ceiling 32 KiB,
unknown fields rejected. Every result no-store; generated x-request-id. Errors
are application/problem+json with type=about:blank, HTTP title/status, stable code
and matching request_id. No input values, secrets or database errors echoed.

- GET /v1/application: tenant_id, application_id, api_version, features.
- POST /v1/auth/sign-ins {email,password}: completed login only, 200
  {user:{id,tenant_id,application_id},session:{format:"opaque",credential,
  idle_expires_at,absolute_expires_at}}. Times are integer Unix seconds, exclusive
  deadlines. No redundant success discriminant or speculative challenge states.
  Future challenges need a separate versioned operation; unknown format fails safe.
- GET /v1/sessions/current: same user and session deadlines/format, no credential.
  Inspection is protected verification; read-only, never renews.
- POST /v1/sessions/current/renew: same inspection result, no new credential;
  explicit qualifying activity extends idle to min(DB now+86400,absolute).
- POST /v1/sessions/current/revoke: 204, idempotent even unknown/wrong-app session;
  invalid app authority remains 401. Other devices/apps unaffected.

400 invalid_request; 401 invalid_application_credentials, authentication_failed,
unauthenticated; 413 payload_too_large; 415 unsupported_media_type; 429
rate_limited with Retry-After; 503 temporarily_unavailable. Unknown user, wrong
password, unverified/disabled/ineligible account all give authentication_failed.
No session issuance on any failure. SDK validates success shapes/scope and format,
bounds time/response size, disables redirects and automatic retries, omits secrets
from thrown errors. Server imports only; separate client exports contain no config.
Browser responses explicitly project authenticated boolean, user ID/deadlines only.

Next.js adapter owns canonical configured origin, HTTPS except explicit loopback
HTTP development; no forwarded-header authority. Mutations require exact Origin,
JSON and an expiring random CSRF token bound to an authenticated-encrypted HttpOnly
pre-auth cookie. CSRF transaction replaced after sign-in. No wildcard CORS. Only
fixed /protected return destination in this slice, eliminating redirect input.
AES-256-GCM cookies bind origin, app, cookie purpose and key ID as AAD. Random
96-bit nonce, bounded payload, keyring, current encrypting key with old decrypting
keys. Host-only HttpOnly SameSite=Lax Path=/, Secure deployment __Host- name;
separate localhost development cookie. Never persist app secrets in browser.
Cookie deadline is absolute maximum; backend exclusively enforces idle. This lets
explicit renewal work without cookie rewrites or races. Server Components verify
read-only; renewal only at writable CSRF-protected route boundaries.

## Lifecycle / concurrency / threats

PostgreSQL is authoritative; no positive cache or replicas. Transactions use a
shared database advisory transaction lock for all identity/session/provisioning
changes and verification. This intentionally coarse first implementation orders
cross-instance operations and prevents eligibility/revoke vs renewal races.
DB time is sampled after lock acquisition; expired iff now >= either deadline.
No client clock/skew grace for service authorization. Locks/queries bounded;
connection or transaction failure is unavailable, never signed-out. Recovery
opens a fresh connection per operation; no permanently broken shared client.
Throughput under contention is a recorded limit, not an enterprise-scale claim.

Opaque credentials contain 256 random bits. Store only SHA-256 digests; same for
high-entropy application secrets. Argon2id passwords with random salts and PHC
parameters m=19456 KiB,t=2,p=1; dummy verification for unknown identities. Passwords
15..128 UTF-8 bytes at provisioning, never normalized/truncated. Email policy for
this profile is ASCII, trim/lowercase, <=254 bytes, exactly one @ and nonempty
parts; not an email-ownership proof. Provisioning asserts synthetic verification.

Stable credential for the bounded session lifetime: no rolling rotation or old
credential grace/reuse alarm. Theft permits bearer replay until revocation or
expiry; TLS, encrypted host cookie, app authority, CSRF and online checks reduce
exposure but cannot defeat a compromised app server/XSS. Rotation alone does not
solve bearer replay. Routine renewal is idempotent in authority (may advance idle
on later valid retry), never absolute expiry. Concurrent renewals serialize;
lost response or failed cookie rewrite leaves the credential usable. Inspection
does not renew, so passive polling cannot extend sessions. Apps explicitly renew
on foreground protected activity, no background keepalive.

Sign-in response loss: no automatic retry, possible orphan session expires at idle
24h / absolute 7d; subsequent explicit login creates independent device session.
Cookie encryption/persistence preparation failure triggers best-effort revoke;
network disconnect/browser rejection after Set-Cookie cannot be detected. Browser
receives success only after constructing response with cookie; protected request
proves persistence. No distributed transaction or exactly-once login claim.
Sign-out revokes first and clears cookie only after confirmed upstream result;
outage returns 503 and preserves cookie for retry. Revocation commit blocks later
verifications; previously authorized in-flight application work cannot be recalled.
An overlapping verification is ordered by lock acquisition. No time-based
zero-latency promise. Account/app/enrollment removal denies next check; reenabling
must not revive existing sessions: operator removal also revokes affected rows.

Sign out everywhere is separate and not exposed/advertised in this slice.
App secret rotation accepts explicitly provisioned current/previous digests for
bounded rollout, then removes old; compromise requires removing compromised key
and revoking that app's sessions. Cookie key rotation deploy readers first, switch
writer second, retain old readers up to seven days, then remove; emergency removal
forces local login but does not revoke upstream: revoke app sessions separately.
Missing/invalid key config fails startup. DB backup rollback can resurrect revoked
sessions: after restore revoke all restored sessions before serving; don't claim
recovery safety without that operation. No signing keys for opaque credentials.

Cleanup deletes expired/revoked sessions and expired abuse buckets; validity never
depends on cleanup. Operator command runs explicitly, no unbounded autonomous
scheduler. Durable rate buckets cover app and normalized account at app/tenant
scope before hashing (30/app/minute and 5/account/15min), including unknown emails;
not raw forwarded IP. Bounded process admission limits hashing pressure. Edge
source limits, abuse tuning, capacity/HA, TLS database deployment, managed keys,
backup drills and operational recovery evidence remain deployment prerequisites.

## Acceptance / evidence obligations

Repeatable PostgreSQL tests: repeat migration/provisioning conflicts, two tenants
and two apps, missing enrollment, invalid/wrong/disabled/unverified password,
expiry boundaries, renewal absolute cap, replay, concurrent instances, current
sign-out isolation, eligibility removal, outages/reconnect and cleanup/recovery.
SDK/adapter: malformed/unknown response fail closed, secrets absent browser output,
CSRF/origin rejection, cookie tamper/key rotation, renewal/lost-response behavior,
protected reload and sign-out. Private demo must pass fresh install/build and
browser/direct HTTP integration using synthetic identities and ephemeral secrets.
Record actual coverage and failures separately; tests do not establish readiness.
