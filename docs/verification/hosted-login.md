# Hosted login integration evidence — 21 November 2026

Observed local Linux x86_64 evidence using the pinned toolchains and disposable
PostgreSQL. User authorized an orchestrator with three delegated scopes:
independent security/contract review, Rust backend/tests, SDK/Next.js/private demo.
The parent reviewed consistency, ran combined checks and recorded evidence.
No provider fixtures, populated provider env files or organization implementation
were changed. No publication, deployment or production-readiness claim.

## Design and reviewable local slices

- `1b51347`: reviewed hosted transaction gate, recorded in Confluence before routes.
- `f31d7c7`: documented browser-policy findings and shared cancellation correction.
- `845b429`: migration, operator callbacks, hosted password/code/exchange backend,
  shared durable admission and cancellation hardening.
- `9107d39`: typed server SDK/Next.js integration, runtime validation, tests and
  machine-readable hosted contract.
- `11b879c`: normal setup, both profiles and failure/recovery documentation.
- Private `kanche-auth-research` revision `ef13507`: both runnable profiles,
  direct-HTTP client and actual Chromium evidence, separate from provider fixtures.

All revisions are local and unpushed. [Contract/lifecycle](../architecture/0004-hosted-login.md),
[setup guide](../contracts/hosted-integration.md), [OpenAPI](../contracts/hosted.openapi.json).

## Results

| Check                             | Observed result and scope                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| --------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Combined `make check`             | Passed: Rustfmt/Clippy with denied warnings, Oxfmt/Oxlint, relative links, four Rust unit tests, two foundation tests, binary lifecycle checks, TypeScript, four SDK/adapter tests, both OpenAPI 3.1 documents, both disposable PostgreSQL scenarios, Rust docs. DB scenarios are ignored by plain cargo test and explicitly run by make check.                                                                                                                                                                                         |
| Rust unit cases                   | Hash worker retains its owned permit after awaiting task cancellation; canonical callback/origin and IPv6 allow/reject; strict form percent/UTF-8 decoding; CSP source delimiter encoding.                                                                                                                                                                                                                                                                                                                                              |
| PostgreSQL hosted scenario        | Actual v1-to-v2 migration retains existing account/session, checksum failure/recovery, repeated registration, malformed/duplicate/context/origin/CSRF inputs, generic password failure,413/415 headers, invalid UTF-8 before attempt counting, no session before exchange, bad app/tenant/callback/proof nonconsumption, transaction/code expiry, concurrent password submission and exchange, single use/replay, enrollment removal/reenable, callback removal/readd, revoke-all and cleanup. Existing app-owned scenario also passed. |
| SDK/adapter                       | Strict hosted URL/scope/grant checks; state/PKCE in encrypted purpose-bound cookie; malformed/duplicate callback query, missing/tampered/expired pending cookie, stale attempt supersession, key rollover/removal,503 preserving cookies,303 clean protected redirect, no automatic exchange retry and replay failure.                                                                                                                                                                                                                  |
| Private demo working checkout     | Production build/typecheck passed. Full direct-HTTP and Chromium harness passed both profiles, protected reload, explicit renewal/current sign-out, CSRF/invalid password, API outage and recovery. Hosted password never appeared in app POST requests.                                                                                                                                                                                                                                                                                |
| Actual hosted callback outage     | Intercepted the native Kanche password POST response after committed303 with redirects disabled, stopped the API, then delivered that unchanged redirect response to the browser. App callback returned503 and retained correlation; API restart plus explicit reload completed exchange/protected access. Replayed callback returned400 without clearing the active app session. This proves the pre-exchange outage case, not transparent replay of a committed lost exchange result.                                                 |
| Fresh authorized core checkout    | Passed make setup/check/security from a fresh authorized local clone with a fresh Rust target directory. Package/dependency and browser download caches remained available; this is not a fresh-OS or network-independent claim.                                                                                                                                                                                                                                                                                                        |
| Fresh authorized private checkout | Passed npm ci, production build, typecheck and the complete HTTP/Chromium harness from a fresh authorized sibling clone. Dependency and downloaded headless Chromium caches were reused; no populated env files/provider accounts were needed.                                                                                                                                                                                                                                                                                          |
| Security                          | Working and fresh checkout make security passed: redacted scans no findings, advisory/source/license checks passed and npm audits reported zero vulnerabilities. Redacted staged private-source scan found no leaks. Nonblocking cargo-deny duplicate-version/unused-allowance warnings remain; no advisory bypass.                                                                                                                                                                                                                     |

## Failures, findings and corrections

1. Initial native Chromium form submitted `Origin: null` under document
   `Referrer-Policy: no-referrer`, producing403. Direct HTTP had passed because it
   manually supplied Origin. Hosted form/retry documents now use `same-origin`;
   handoff303 and app callback keep `no-referrer`. Exact Origin, anonymous browser
   binding and CSRF remain enforced; null Origin is rejected. Same-origin Referer
   can include the transaction handle and requires ingress/APM redaction.
2. Chromium then blocked the required cross-origin303 under `form-action 'self'`.
   Trusted typed form context now lists self plus that transaction's registered
   callback source. Origin validation and percent-encoded path prevent CSP directive
   injection. Initial and retry forms use the same policy; missing trusted context
   yields a restart-only error. The HTML form action remains fixed to Kanche.
   CSP skips path matching after redirects: exact backend registration/Location and
   app state/PKCE remain the authority. No wildcard/global callback allowance.
3. The outage harness initially tried to intercept a redirect destination with
   page.route; it did not stop the API, so the callback correctly returned303
   instead of the expected503. Moved interruption to the actual committed native
   POST303 response. No authentication checks were weakened to satisfy the harness.
4. Independent review found request cancellation could release a borrowed hash
   permit while work continued and roll back attempt counters. Admission now commits
   before hashing, an owned permit stays inside the worker, and final scope/hash/
   eligibility checks run under authoritative ordering. Hosted pending context is
   rechecked too. Deterministic unit evidence covers worker permit lifetime; full
   interrupted-HTTP plus DB-accounting fault injection remains unexercised.
5. Replaced lossy hosted form decoding with strict UTF-8/percent decoding; fixed
   IPv6 loopback host parsing; applied security headers to extractor/method/timeout
   failures. An eight-argument test helper initially failed denied-warning Clippy;
   a typed method/path tuple fixed it without a lint exclusion.

Diagnostics used safe stage/status and boolean policy observations, never raw
callback URLs, console violation text, password bodies, tokens or cookie values.
No captures, screenshots, traces or browser state were persisted. Harness-owned
processes and disposable database containers were cleaned up after each run.

## Limits and remaining roadmap

The required app-owned and hosted profiles share the same direct enrollment and
online app-bound opaque sessions,24h idle/7d absolute, foreground renewal and
current-session sign-out. KN-2's two-profile initial contract is complete; KN-4
public signup/email verification, KN-10 recovery, KN-11 organization ownership/
grants and future JWT purpose/timing remain separate work. No organization schema,
claims, speculative grants or cross-app SSO were introduced. Operator revoke-all
is recovery, not a delivered user sign-out-everywhere API.

Only the localhost-app/127.0.0.1-Kanche HTTP browser profile is exercised. IPv6 URL
validation is unit-tested; literal-IP/IPv6 callback CSP/browser support and other
browser engines are unverified. No remote database TLS, deployed HTTPS/managed
keys, edge abuse tuning, capacity/HA or backup-restore drill is established.
Pending-code disable-user/disable-app/key-removal combinations are implemented but
not individually exercised; their session/operator counterparts have app-owned
coverage. This is not an exhaustive failure/concurrency matrix.

Codes are single-use. A lost password-success response requires restart; a lost
committed exchange result may leave a bounded orphan and cannot be transparently
replayed. Explicit restart, expiry/cleanup and operator recovery apply. Browser
cookie rejection/network-write failures and every key rollout topology are not
fault-injected. Restoring a backup requires invalidating all restored hosted
transactions and revoking restored sessions before protected traffic. Mixed-version rolling/rollback drills are unverified; apply all migrations before
upgrading code, use the new operator for removal/recovery, and invalidate pending
handoffs before rollback. Coarse
cross-instance authority locking and synchronized clocks remain operational limits.
