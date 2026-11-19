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
applicable. These flows are not implemented yet.

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

Repository boilerplate does not satisfy these design gates. The next design
scope is the native integration contract; no authentication or session routes
are authorized by this planning index.
