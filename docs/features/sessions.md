# Sessions

**Status:** Planned, Tasks 06–09. No Kanche session routes exist.
The [V1 roadmap](../architecture/v1-roadmap.md) requires a design gate before
implementation. The [integration guide](integration.md) describes the native
contract that must be reviewed before implementation.

## Session design gate

Task 06 must define browser cookies, token boundaries, origins, application
binding, expiry, renewal, sign-out scope, revocation, and signing-key rotation.
Record the lifecycle and threat model for review. Cookie names and attributes,
claim formats, token lifetimes, key publication, and revocation latency are not
specified yet. Each behavior needs an explicit contract and repeatable checks.

## Issuance and protected requests

Task 07 issues an app-bound session after authentication and validates it through
the selected integration. Demonstrate a protected page through the ordinary
developer setup. Reject cross-app and cross-tenant session use. Specify the
signed-out response and return destination as part of the integration contract.

## Renewal

Task 08 handles expiry and renewal, retries, concurrent requests, and replay.
Verify behavior at expiry and under overlapping renewals. The design must state
what happens after retryable or partial failure. Foreground refresh and fresh
navigation with an expired token are separate scenarios; their behavior must
follow the approved contract.

## Sign-out and revocation

Task 09 defines user sign-out and administrative revocation scopes, clears or
invalidates the appropriate session state, and verifies subsequent protected
requests. Record the effect on already-issued tokens, including any remaining
validity window. Do not assume deleting a browser cookie invalidates every token.

[Password recovery](authentication.md#password-recovery), account disablement,
[organization removal and app grants](organizations.md) must use the approved
lifecycle and have separately tested scopes.
