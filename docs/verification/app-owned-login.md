# First app-owned login evidence — 21 November 2026

Observed local Linux x86_64 evidence, using Node 26.7.0/npm 11.19.0,
Rust 1.99.1 and Docker 29.1.3. No publication, deployment or readiness claim.

## Reviewable local revisions

- `efb6428`: KN-2/KN-6 app-owned contract and lifecycle design prerequisites,
  recorded before implementing routes. See the
  [design decision](../architecture/0003-app-owned-login.md).
- `5c859fa`: PostgreSQL provisioning/password/session slice (KN-3/5/7).
- `832413c`: server SDK, Next.js adapter, OpenAPI and integration checks (KN-2/8/9).
- `8f9ac69`: setup, maintenance, recovery and scope documentation.
- Private `kanche-auth-research` revision `aba0eb1`: separate first-party
  `kanche-demo`, direct-HTTP client and Playwright evidence harness.

These are local commits; nothing was pushed. Provider fixtures were preserved.
[Setup and API guide](../contracts/app-owned-integration.md).

## Results

| Check                                   | Observed result / scope                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| --------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `make setup`                            | Passed in fresh authorized local checkout; locked npm install and Rust dependency fetch.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| `make check`                            | Passed: Rustfmt, Clippy with denied warnings, Oxfmt/Oxlint, relative doc links, two foundation tests, binary startup/redaction/signals/restart checks, TypeScript checks, two SDK/adapter suites, OpenAPI 3.1 validation, disposable PostgreSQL invariant scenario and Rust docs. The DB scenario is ignored by plain cargo test and explicitly executed by make check.                                                                                                                                                                                                                      |
| PostgreSQL scenario                     | Repeat migration/provisioning and tenant conflict; two tenants/two apps; invalid/unknown/unverified/disabled/unenrolled login; app metadata authority; cross-app/tenant credential rejection; no credential in inspection; simultaneous independent-store renewal; original bearer works after lost renewal response; absolute cap; idle/absolute boundary; current-device/app isolation; next-check eligibility; enrollment/user/app removal prevents revival; concurrent revoke/renew; durable account throttling; lock-timeout unavailable and recovery; key removal; revoke-all/cleanup. |
| SDK/adapter suites                      | Scope and format rejection, bounded response, redacted transport failures and serialization, CSRF/origin checks, encrypted HttpOnly host-only cookie, tamper rejection, old/new key reader rollout, Secure deployment cookie attributes, cookie retained on unavailable sign-out and cleared after confirmation.                                                                                                                                                                                                                                                                             |
| `make security`                         | Passed: redacted directory secret scan (no findings), advisory/source/license checks, npm audit (zero reported vulnerabilities), dependency licenses. Cargo-deny reports nonblocking duplicate-version and unused-allowance warnings.                                                                                                                                                                                                                                                                                                                                                        |
| Private demo `npm ci`, build, typecheck | Passed using pinned Next.js 16.4.6, React 19.3.1, TypeScript 7.0.2, Playwright 1.65.1.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| Private `npm run evidence`              | Passed actual API direct HTTP and Chromium browser login/protected reload/explicit renewal/sign-out. Invalid password and CSRF rejection, safe response projection, HttpOnly browser cookie, actual stopped-API HTTP 503 with no cookie clearing, and restart/recovery with the same cookie passed.                                                                                                                                                                                                                                                                                          |
| Fresh authorized checkouts              | Locally cloned both committed repositories into fresh sibling directories without populated env files. All setup/check/security and private install/build/typecheck/evidence commands above passed on retry. Existing Rust dependency/build caches and downloaded headless Chromium were reused; this is not a cold-cache/network-independent test.                                                                                                                                                                                                                                          |

The PostgreSQL lock-timeout test intentionally logs SQLSTATE 55P03 without SQL,
parameters or credentials. A single scenario covers multiple invariants; its
passing test count is one, not a claim of independent exhaustive coverage.

## Failures encountered and recovery

- Initial debug password login returned unavailable under the bounded execution
  path. Optimizing Argon2/Blake2 dependencies in the dev profile made the normal
  developer flow pass; hash parameters and request/DB bounds were retained.
- Node strip-only TypeScript rejected constructor parameter properties; replaced
  them with ordinary fields. Next.js initially could not resolve sibling packages;
  configured the demo's Turbopack root to include both authorized checkouts.
- First browser attempt lacked its pinned Chromium executable. Installed it and
  reran successfully. No provider browser flows or broad research were repeated.
- Initial fresh-checkout attempts hit disk quota (OS error 122) during Rust build,
  Next.js dependency extraction and security-tool installation. Removed only
  session-created duplicate artifacts, moved the fresh checkouts off size-limited
  `/tmp`, and reused the Rust target cache. Complete reruns then passed. Sufficient
  local storage is required; a fully cold build was not verified.
- New workspace entries needed explicit original-code license validation. Reviewed
  argparse's PSF-2.0 notice/changes terms (OpenAPI validator dependency), retained
  its bundled license and allowed that exact expression. No package was modified;
  no advisory bypass was added. Formatting/lint/type errors were fixed before the
  passing runs recorded above.

## Remaining limits / next delivery

App-owned first-profile design gates are complete; KN-2 remains open for the
required hosted profile's detailed transaction/callback contract. Hosted login is
next, with the same app enrollment/session policy. KN-4's public signup/email
verification and KN-10 recovery are not delivered; only their provisioned-account
prerequisite is present. KN-11 remains an open design gate, with no organization
schema/claims/routes. Future JWT purpose/timing remains open. User sign out
everywhere remains separate and unadvertised; database-wide operator revoke-all
is for recovery, not that product API.

Local HTTP only; deployment TLS/remote PostgreSQL TLS, edge abuse admission,
capacity/HA, managed keys, operational metrics and backup-restore drills remain
unverified. Global transaction locking is intentionally coarse. Stable bearer
replay remains possible with valid app authority until expiry/revocation. Revoked
or ineligible sessions reject at the next ordered authoritative check; in-flight
application work may finish. Cookies require synchronized server clocks; no skew
grace is provided. The browser/SDK deadline can precede service completion, so
lost login responses or rejected cookies may leave bounded orphan sessions.
No exactly-once issuance, cookie-persistence acknowledgment or exhaustive injected
cookie-write/network-failure evidence is claimed. Cleanup/revoke-all commands are
tested; an actual database backup/restore procedure has not been drilled.
