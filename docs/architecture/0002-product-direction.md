# Kanche product direction

**Status:** Accepted direction; implementation contracts require review.

Kanche defines its own identity, authentication, session, and authorization
contracts. Build straightforward developer integration, explicit tenant and
application boundaries, predictable failure behavior, and repeatable evidence
for the guarantees the service claims.

## Product boundaries

A workspace manages identity tenants and applications. Tenant identities and
application access have distinct lifecycles. Organization ownership, grants,
roles, and active context remain proposals until their design gate is reviewed.
The [SaaS model](saas-plan.md) documents those proposals.

The native integration must specify browser/server responsibilities, origins,
redirects, configuration, safe errors, and the supported developer entry point.
Session work additionally requires an approved lifecycle and threat model.
Standards conformance requires evidence against the relevant specification.

## Planning and delivery

Detailed discussions live in the
[Kanche Confluence space](https://fyndx.atlassian.net/wiki/spaces/KA), and delivery
work is tracked in the [Kanche Jira project](https://fyndx.atlassian.net/issues/?jql=project%20%3D%20KN).
The [design gates](v1-roadmap.md) remain prerequisites to implementation.
Record the concrete flow, contract, tradeoffs, failure behavior, migration
implications, and validation criteria before implementing a new boundary.

The current service exposes only the health endpoint. Planning documents and
repository checks do not establish that authentication or sessions are shipped
or production-ready.
