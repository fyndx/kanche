# Hosted password login integration

The required second native profile redirects the browser to Kanche for password
entry, then exchanges a short-lived one-use handoff on the app backend. The
[reviewed design gate](../architecture/0004-hosted-login.md) and
[OpenAPI 3.1 contract](hosted.openapi.json) define the boundaries and recovery.
This complements [app-owned login](app-owned-integration.md). Both use the same
provisioned accounts, direct enrollment and online app-bound session lifecycle.
No shared Kanche login/SSO, organizations, JWT, signup or recovery API is introduced.

## Configuration and callback registration

Apply all operator migrations before enabling hosted routes. Set
`KANCHE_HOSTED_ORIGIN` explicitly to the canonical externally visible service
origin. HTTPS is required except explicit loopback HTTP development. Omitting the
setting leaves hosted routes disabled. Incoming Host/forwarded headers cannot
select this authority. The initial SDK uses the same canonical public origin for
API and hosted pages; separate internal API routing is not supported in this slice.

Use `cargo run --locked --bin kanche-admin -- register-callback` with bounded
stdin JSON {application_id,redirect_uri,development?:boolean}; `remove-callback`
uses the same shape. These operator commands need the database environment only.
For the private demo
this is `http://localhost:3010/api/kanche/callback`, with `development:true` to
explicitly allow loopback HTTP. Registration contains no application secret and
cannot be performed with an app bearer. No wildcard, prefix, query, fragment,
userinfo, backslash, controls or noncanonical callback spelling is accepted.
Removing a callback invalidates related outstanding transactions. Re-adding it
must not revive old handoffs.

Configure the Next.js adapter's optional `hostedCallbackUri` to the exact app
origin plus `/api/kanche/callback`. Keep `baseUrl`, `applicationSecret`, `tenantId`,
`applicationId`, canonical `origin`, cookie keyring/current key and explicit
loopback `development` settings from the app-owned integration. The callback
correlation cookie is encrypted separately by purpose under that keyring. It
contains random state, PKCE verifier and a bounded expiry; neither value is a
Kanche app-session credential.

## Browser and server flow

1. The browser obtains the existing expiring CSRF transaction and POSTs same-origin
   JSON to the app adapter's `hosted-start` operation. App Origin and CSRF checks
   apply to this login entry point too.
2. The app generates state/PKCE, creates a five-minute Kanche transaction using
   its app secret and registered callback, seals its correlation cookie, and sends
   the browser to the validated authorization URL. The SDK validates that URL
   against its configured service origin, fixed `/hosted/login` path and one
   random transaction parameter.
3. Kanche renders a no-script password form with a separate anonymous HttpOnly
   browser nonce and transaction-scoped CSRF token. Password submission remains
   on Kanche's configured origin. No password is sent through the app backend.
4. Password success creates a 60-second handoff code, with no session yet, then
   redirects 303 to the registered callback with code/state. Invalid passwords
   show a generic retry form; invalid browser context never causes an arbitrary
   redirect. The anonymous Kanche cookie does not authenticate future logins.
5. The app callback checks exact query shape and browser correlation/state before
   authenticated PKCE exchange. Kanche checks current user/app/enrollment again,
   consumes the code atomically and issues the ordinary app-bound session.
6. The adapter seals that credential using the existing session-cookie policy,
   clears the transaction cookie after success and immediately redirects 303 to
   fixed `/protected`. Online protected verification, foreground renewal and
   current-session sign-out then work identically for either profile.

There is one pending hosted attempt per app/browser. Starting another replaces
its app correlation cookie; callbacks from older attempts require restart.
Anonymous nonce races during simultaneous first page loads may also require
restart. These limits do not restrict multiple signed-in devices.

## Failure and security behavior

Wrong app, callback or PKCE proof does not consume a legitimate code. A valid code
can issue at most one session, including across concurrent instances. Code replay
fails with `invalid_handoff`. Expiry is enforced by the authoritative database,
independently of cleanup. Operator enrollment/user/app removal, callback removal
and recovery revoke-all invalidate affected pending handoffs as well as sessions.

The SDK does not automatically retry creation/password/exchange. Lost password
success response requires a new login. A lost exchange response may leave a
bounded orphan session if the commit succeeded: a retry cannot replay that result,
and the user restarts login. Dependency failure is unavailable/503 and preserves
existing session and transaction cookies; it is never an invalid-password result.
Detectable cookie-preparation failure triggers best-effort upstream revoke.
No distributed exactly-once response or browser cookie persistence acknowledgment
is claimed.

Short-lived code/state/transaction query parameters are deliberately permitted for
this redirect protocol; raw session credentials and app secrets are never in URLs.
Do not log raw queries, Location headers, form bodies or cookies in ingress/APM.
Hosted form/retry pages use same-origin Referrer-Policy; successful handoff303
and app callbacks use no-referrer. Chromium otherwise submits Origin:null under
a no-referrer form document, so accepting null is deliberately forbidden. All
responses use no-store/nosniff plus restrictive CSP,
with no external scripts/assets or reflected account input. Hosted form/retry CSP
allows only self and that transaction's exact registered callback source: Chromium
otherwise blocks the required cross-origin303 under self-only form-action. The
form's action is still fixed to Kanche; no wildcard or global callback allowance
is used, and the callback source is safely encoded to prevent directive injection.
CSP path matching is skipped after redirects, so exact backend callback registration
and app state/PKCE remain the authority; CSP alone does not enforce callback-path
equality. Literal-IP/IPv6 browser callback CSP support remains unverified. Exact configured
origins, app authority, state, PKCE, anonymous browser binding and CSRF each enforce
a different boundary; none replaces the others.

## Private runnable evidence and limits

Use authorized sibling checkouts and the pinned prerequisites described in the
[app-owned setup](app-owned-integration.md). The private `kanche-demo` includes both
profiles and a direct-HTTP client; no provider accounts or populated env files are
needed. Synthetic identities and keys are generated in memory and the disposable
PostgreSQL container is removed afterward. App and Kanche use distinct loopback
hosts (`localhost:3010` and `127.0.0.1:18080`) to exercise cookie isolation.

```sh
make setup
make check
make security
cd ../kanche-auth-research/kanche-demo
npm ci
npx playwright install chromium --only-shell
npm run build
npm run typecheck
npm run evidence
```

See the verification record for actual passing scope and gaps. Local checks do
not establish deployment TLS, remote database TLS, edge abuse resistance,
capacity/HA, managed-key operations, browser-matrix coverage or backup/restore
readiness. The database-wide authority lock remains intentionally coarse. Restore
must invalidate outstanding hosted handoffs and revoke restored sessions before
protected traffic. KN-11 and future JWT purpose/timing remain separate design work.
