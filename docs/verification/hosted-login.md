# Hosted login verification

## Repeatable checks

Run `make check` with pinned toolchains and Docker. The hosted PostgreSQL scenario
covers migration, callback registration, browser/CSRF/PKCE binding, app and tenant
isolation, expiry, replay, concurrent exchange, eligibility removal, and cleanup.
SDK/adapter tests check state and PKCE correlation, encrypted cookies, response
validation, and preservation of correlation on unavailable exchange.

Additional regression checks cover the 60-second hosted-creation retry response,
unknown maintenance targets, app-scoped revocation, and bounded database admission.
`make security` checks secrets, dependency advisories, and licenses.

## Browser policy

Hosted forms use `Referrer-Policy: same-origin` so browsers can send the required
Origin. Redirects and callbacks use `no-referrer`. Same-origin Referer can contain
the transaction handle, so ingress/APM must redact it. Form CSP permits only the
trusted registered callback source; exact backend registration, state and PKCE
remain authoritative. Repository HTTP tests do not establish full browser support.

## Coverage limits

The database suite combines multiple invariants into one scenario and is not an
exhaustive failure/concurrency matrix. Full interrupted-HTTP plus database attempt
accounting fault injection is not covered. IPv6 URL validation is unit-tested;
IPv6 callback CSP behavior and a full browser matrix remain unverified.

Deployment HTTPS, remote database TLS, managed keys, edge abuse tuning, capacity/HA,
and backup/restore drills are not established. Codes are single-use: a lost password
success requires restart, and a committed exchange with a lost response may leave
a bounded orphan session. Transparent replay is not supported. Cookie rejection,
network-write failures, and every key rollout topology are not fault-injected.

Restore requires invalidating restored hosted transactions and revoking sessions
before protected traffic. Mixed-version rolling upgrade and rollback drills remain
unverified. Apply migrations before upgrading; invalidate pending handoffs before
rollback. Shared authority locking and synchronized clocks remain operational limits.

See the [hosted guide](../contracts/hosted-integration.md) for the public contract.
