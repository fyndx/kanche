# Authentication and account lifecycle

**Status:** Planned, Tasks 04, 05, and 10. No authentication routes exist.
Depends on the integration contract and tenant/application foundation in the
[V1 roadmap](../architecture/v1-roadmap.md).

## Signup and verification

Task 04 defines account creation, identifier normalization, password handling,
and verification through the tenant's managed password connection. The intended
flow is signup, verification delivery, and verification completion. Exact API
shapes, delivery provider, token policy, and retry rules remain to be designed;
delivery and retries may need a separate follow-up task.

Acceptance evidence must cover the normal flow, duplicate accounts, expired
verification, and replay. Password and verification material must stay out of
logs and committed fixtures. Safe error behavior and abuse controls must be
specified without leaking account secrets.

## Password sign-in

Task 05 verifies credentials using the managed connection and defines abuse
controls and safe failures. Verify valid credentials and invalid, unverified,
and disabled accounts. Credential verification precedes
[session issuance](sessions.md#issuance-and-protected-requests); completing this
task alone does not establish an authenticated browser session.

## Password recovery

Task 10 delivers a reset, validates a one-time token, and changes the password.
Define the effect on existing sessions before implementation. Verify the normal
recovery path, expiry, replay rejection, and the selected session policy. Delivery
and retry behavior must have predictable failures and actionable, safe errors.

## Account disablement

Sign-in must handle disabled accounts. The proposed SaaS lifecycle makes account
disablement tenant-wide, affecting access across that tenant's apps. Exact
administrative APIs and effects on already-issued sessions/tokens require the
[session gate](sessions.md#session-design-gate); immediate revocation is not an
implemented guarantee. Account disablement, organization removal, and app grant
revocation have different scopes.
