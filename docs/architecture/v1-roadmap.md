# Kanche V1 planning and design gates

Detailed planning and design discussions live in the
[Kanche Confluence space](https://fyndx.atlassian.net/wiki/spaces/KA).
Delivery tasks live in [Jira project KN](https://fyndx.atlassian.net/issues/?jql=project%20%3D%20KN).
Existing planning was preserved in the
[planning archive](https://fyndx.atlassian.net/wiki/spaces/KA/pages/97583105).
Migrated task descriptions require refinement against the current product
direction; their presence in Jira does not approve a design or prove completion.

Work on one named, reviewable scope at a time. Record decisions, implementation
behavior, verification, and remaining limits in the applicable Jira issue and
Confluence design page.

## Foundation and first integration

Review the [native integration contract, KN-2](https://fyndx.atlassian.net/browse/KN-2)
before implementing authentication routes. Specify browser/server boundaries,
origins, redirects, developer configuration, and safe errors.

Tenant/application provisioning, password signup, verification, sign-in, and
recovery are tracked in KN-3, KN-4, KN-5, and KN-10. Preserve tenant/application
isolation and verify failure, expiry, replay, retry, and recovery behavior as
applicable. The first operator-provisioned password login slice is implemented; public signup,
verification delivery and recovery remain open. See the
[contract and workflow](../contracts/app-owned-integration.md).

Review the [session lifecycle and threat model, KN-6](https://fyndx.atlassian.net/browse/KN-6)
before session issuance, renewal, or revocation work in KN-7 through KN-9.
Decide app binding, cookies/tokens, expiry, concurrent renewal, sign-out,
revocation latency, and signing-key rotation before implementing them.

## Organizations and app access

Review [organization ownership and access, KN-11](https://fyndx.atlassian.net/browse/KN-11)
before implementing organization tables, endpoints, or session claims. The
[SaaS proposal](saas-plan.md) remains a proposal. Membership/invitations and
application grants/context are tracked in KN-12 and KN-13.

## SaaS onboarding and release

Workspace/console access, self-service provisioning, and release readiness are
tracked in KN-14 through KN-16. Release evidence must cover the normal developer
flow, isolation, configuration, migrations, operational visibility, and recovery.
Billing, configurable connections, and dedicated hosting remain deferred.

The app-owned KN-2 and KN-6 engineering prerequisites are completed in the
[concrete contract and lifecycle](0003-app-owned-login.md), within the latest
user-approved defaults recorded in Confluence. The first login slice implements
that bounded profile. The required [hosted redirect contract](0004-hosted-login.md)
completes its separate callback/state/PKCE transaction prerequisite; the
[hosted integration](../contracts/hosted-integration.md) preserves the same app-bound
policy. Future JWT purpose/timing and KN-11 remain open. Repository boilerplate
and passing local checks do not prove production readiness.
