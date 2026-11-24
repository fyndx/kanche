# Hosted login architecture

Extends [the app-owned lifecycle](0003-app-owned-login.md) with a browser-bound,
single-use handoff. This architecture description is separate from test evidence.

## Trust and transaction contract

The app backend authenticates with its separate application secret, generates
state (32 random bytes, 64 lowercase hex) and PKCE verifier (32 random bytes,
64 lowercase hex; service accepts RFC7636 43..128 unreserved ASCII). Only S256:
base64url(SHA256(ASCII verifier)) without padding, exactly 43 characters.
No OAuth/OIDC interoperability or cross-app SSO is claimed. Hosted Kanche owns the
password page; the app never receives that profile's password. Existing app-owned
login remains supported and both profiles share password/eligibility/rate/session
policy. No organization behavior or JWT format is introduced.

1. Operator registers an exact callback for one app using bounded stdin JSON
   {application_id,redirect_uri,development?:boolean}. HTTPS required except explicit loopback HTTP
   development enabled by development:true. Reject userinfo, query, fragment, backslash, controls, wildcard,
   ambiguous/noncanonical encodings. Use a fixed absolute callback with an
   absolute non-root path; compare the registered string exactly, no prefix match.
   App credentials cannot register callbacks. Removing a registration invalidates
   its outstanding transactions. No arbitrary return_to in this slice.
2. POST /v1/hosted/transactions, authenticated app, JSON
   {redirect_uri,state,code_challenge,code_challenge_method:"S256"}. Derive tenant/app
   exclusively from app authority; registration must match that app. Return 200
   {authorization_url,expires_at}. URL uses configured KANCHE_HOSTED_ORIGIN and
   /hosted/login?transaction=<random 256-bit handle>. Five-minute DB deadline.
   Only the handle digest is stored. State is stored to return at handoff; it is a
   correlation nonce, never session/application authority. Creation is rate
   limited at 30/app/minute in addition to shared password limits.
3. GET /hosted/login?transaction=... validates the pending, unexpired transaction
   and current app/tenant/registration. Bind it on first load to the digest of a
   random host-only HttpOnly SameSite=Lax anonymous browser cookie. Subsequent
   loads require that browser binding. The anonymous nonce may be reused across
   transactions; it confers no identity or SSO. A fresh independent CSRF token is
   rendered in the form and its digest stored on each GET; refreshing replaces it
   so stale forms must reload. No application secret or session credential in HTML.
4. POST /hosted/login accepts bounded form-urlencoded
   {transaction,csrf_token,email,password}, requires exact Origin equal to configured
   hosted origin, matching anonymous browser cookie and transaction-bound CSRF.
   Invalid authentication returns generic 401 safe HTML with a retry form, no
   reflected email/password; rate failures 429/Retry-After. No session issued yet.
   Password success atomically records user and a hashed random 256-bit code,
   expiring in min(60s,transaction remaining lifetime), and disables further form
   submissions. 303 redirects to the registered callback with only code and state.
   Never 307/308: passwords must not be forwarded. Invalid/expired browser context
   errors never redirect to caller-supplied destinations. Lost password-success
   response requires new login; code cannot be recovered from its digest.
5. App GET /api/kanche/callback requires exactly one code and state and no other
   parameters. Validate encrypted short-lived app transaction cookie and constant-
   time state equality before exchanging. PKCE verifier never leaves app backend
   except authenticated exchange. One active hosted transaction per app/browser:
   a new start replaces correlation cookie; older callbacks fail safely. Existing
   sessions on other devices remain independent.
6. POST /v1/hosted/exchanges, app authority, JSON
   {code,code_verifier,redirect_uri}. In one DB transaction under the existing
   cross-instance authority lock: validate code/deadline, app/tenant binding,
   registered callback, S256 proof, current verified active user/enrollment and
   app/tenant; consume handoff and issue exactly one normal opaque session.
   Return the existing {user,session} server-only Grant. Invalid handoff/proof
   returns 400 invalid_handoff with no account data. Wrong app/callback/proof must
   not consume a legitimate handoff. Repeated/concurrent valid redemption yields
   one success and invalid_handoff thereafter, never another session.
7. The app seals the credential into the existing authenticated-encrypted session
   cookie and redirects 303 to fixed /protected, removing callback code/state from
   the address. Clear transaction cookie only after success. Failures preserve any
   existing app session. Dependency failures return 503, preserve transaction for
   explicit retry/restart, and never claim invalid credentials. No automatic
   exchange retry. If commit happened before response loss, retry is invalid_handoff:
   restart login; bounded orphan session/cleanup applies. Best-effort revoke on
   detectable cookie-preparation failure, as with app-owned login. Browser rejection
   or response loss cannot be atomically acknowledged; no exactly-once completion.

## HTTP, cookies, origin configuration and recovery

Server API remains versioned JSON with 32 KiB extraction bounds, no-store and safe
RFC9457 errors/generated request IDs. Unknown fields and duplicate query/form
parameters reject. No CORS browser authentication API is enabled. Creation,
exchange and HTML/password paths never log raw URLs/query, body, headers, code,
state, cookie or password. Short-lived transaction/code URL parameters are a
narrow reviewed exception to the original no-credential-in-URL guideline: neither
can authenticate an app session without app authority and the PKCE proof. Reverse
proxy/history/Referer exposure remains a deployment risk and must be redacted.

Hosted routes require an explicit canonical KANCHE_HOSTED_ORIGIN (HTTPS, or explicit
loopback HTTP development); no Host/forwarded-header authority. When absent hosted
routes/feature are disabled. It must describe the service itself, not a redirect
proxy. For the first SDK profile baseUrl must equal the canonical public hosted
origin; a separate internal API origin is not supported yet. A loopback development setup uses app http://localhost:3010 and Kanche
http://127.0.0.1:18080, distinct hosts so cookies are not shared merely by port.
Anonymous hosted cookie has a separate Kanche-specific name, Secure deployment
__Host- prefix, Path=/, no Domain, HttpOnly, SameSite=Lax and bounded expiry.
App transaction cookie uses the existing AES-GCM keyring with distinct purpose/AAD,
contains state/verifier/expiry only, and expires no later than the five-minute
transaction. App/session secrets remain separate and server-only. Anonymous cookie GET reuse
reissues the same valid nonce with expiry at most now+5min; each transaction retains
its own DB deadline. Concurrent initial loads without a nonce may conflict and
require restart; arbitrary parallel browser starts are not promised. Key-reader
rollout/emergency removal follow the existing lifecycle and may force restart.

Hosted form/error documents set Referrer-Policy:same-origin; the successful handoff
303 and app callback use no-referrer. Actual Chromium initially submitted Origin:null
when the hosted form document used no-referrer, causing the strict Origin check
to reject a legitimate form. Same-origin policy preserves that exact check while
suppressing cross-origin referrer disclosure; same-origin ingress must still redact
query/header context. Never accept null Origin as a workaround. All hosted and
callback responses set Cache-Control:no-store, X-Content-Type-Options:nosniff
and a restrictive CSP:
no external assets/scripts; default-src 'none', frame-ancestors 'none', base-uri
'none'. Form documents permit form-action 'self' plus only that transaction's
exact registered callback source, including path. Chromium enforces this directive
across the native form's cross-origin303; self-only policy blocked the required
handoff even after Origin was corrected. The HTML action stays fixed /hosted/login.
CSP callback sources must be safely constructed from validated canonical URI and
encoded path so punctuation cannot inject directives; no wildcard or global
callback allowlist. Initial and failed-password retry forms get the same scoped
policy. Generic errors may use self-only when no validated form context exists.
The successful303 may use the safe default; it retains no-referrer. CSP3 skips
source path matching after redirects: the source expression includes the exact
callback path, but CSP does not prove redirect-path equality. Backend exact
registration/Location and app state/PKCE remain authoritative. Literal-IP and
IPv6 callback CSP behavior is not established by the localhost Chromium evidence;
URL validation support is not a browser compatibility claim. Safe hosted renderer may explicitly
preserve its HTML error response through the generic Problem middleware; JSON
API errors remain Problems. Only generated or validated fixed-format transaction/CSRF hidden values may be
HTML-escaped into the form; no email/password/redirect reflection. Invalid contexts
get a safe restart message; no open redirects or caller-reflected form action.

Pending handoffs participate in recovery/revocation, not just issued sessions:
remove-enrollment, disable-user/app and revoke-all invalidate relevant successful-
password handoffs; callback deregistration invalidates its outstanding transactions.
Reenable must not revive a pre-removal code. Registration change, expiry, eligibility
and app-key removal are checked on exchange. New login after reenable is allowed.
Cleanup deletes expired/consumed transactions independently of validity checks.
Restoring a DB requires invalidate all transactions and revoke all sessions before
serving traffic. Existing coarse-lock/contention and DB/TLS deployment limits remain.
Sign-out still revokes only the current app session; no hosted shared login cookie
or automatic cross-app login exists. User sign out everywhere remains undelivered.

## Shared password admission refinement

Independent review found that request cancellation could release a borrowed hash
permit while the blocking hash continued, and roll back rate counters held in the
same transaction. The implementation must commit bounded attempt accounting
before hashing, transfer an owned admission permit into the actual worker until
it exits, then reacquire the authoritative transaction and recheck current app
key, identity/password snapshot and eligibility before issuing a session or
handoff. Hosted submission also rechecks pending transaction/browser/CSRF state.
Cancelled admitted requests consume their rate budget; cancellation cannot create
extra simultaneous hash workers by dropping the awaiting future. Password work
runs outside the coarse DB lock; all authoritative state decisions remain ordered.

## Rationale and acceptance obligations

PKCE plus exact app callback and encrypted browser correlation protect code theft,
app confusion and login CSRF. Separate hosted Origin/browser-cookie/CSRF checks
protect password submission. State-only or app-secret-only handoffs would leave
unnecessary interception/correlation gaps. One-use redemption trades transparent
ambiguous-response recovery for a small auditable contract; explicit login restart
is the chosen recovery, and its orphan limit is acknowledged. This native profile is not an OAuth/OIDC compatibility claim.

Primary references: [RFC7636](https://www.rfc-editor.org/rfc/rfc7636.html) and
[RFC9700](https://www.rfc-editor.org/rfc/rfc9700.html). These inform protections;
they do not establish provider compatibility.

Required evidence: forward migration from app-owned schema, repeat/checksum failure,
operator exact registration and URI attacks; no password at app boundary; CSRF and
browser binding; S256/state/callback/app/tenant isolation; current enrollment and
remove/reenable; exact transaction/code expiry; wrong proof does not consume code;
concurrent password/exchange and replay; lost responses and outage/recovery; safe
HTML/redirects; SDK/adapter redaction/cookie rollover; both existing app-owned and
new hosted browser/direct-HTTP flows. Record gaps and passing scope after
implementation. Deployment TLS, capacity, edge abuse and restore drills remain open.
