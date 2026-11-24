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

## Delivery boundaries

Review concrete contracts, failure behavior, migration implications, and
validation criteria before implementing a new boundary. Detailed planning and
approval records are maintained privately. See the [public roadmap](v1-roadmap.md).

The current branch provides app-owned and hosted provisioned-password login and
online app-bound sessions. Repository checks do not establish production readiness.
