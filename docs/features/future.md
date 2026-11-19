# Deferred capabilities

## Deferred proposals

These items come from the [SaaS proposal](../architecture/saas-plan.md) and remain
outside the [V1 roadmap](../architecture/v1-roadmap.md). They have no committed
release date and do not represent shipped functionality.

## Connections and identifiers

Introduce configurable connections when a second sign-in method requires them.
The proposal considers workforce SSO with an employee app grant, alternative
identifiers such as school-scoped student IDs, and verified account linking.
An identifier names an account; a password or identity provider proves ownership.
Define recovery and linking rules before enabling these flows.

Multiple managed password stores per tenant are deferred. Revisit them for an
app accepting credentials from existing stores that must remain live, with
explicit selection, linking, and recovery rules.

## Billing and entitlements

The proposal separates platform billing, paid by a workspace to Kanche, from
product billing, paid by an end user or organization to the workspace. There is
no implemented billing or committed V1 billing task.

Future product billing would define a product catalog, payment integration,
checkout, subscription lifecycle, cancellation policy, and entitlements. A B2C
subscription attaches to a user; a B2B subscription attaches to an organization.
Multiple app clients could share a product entitlement. Paid entitlement does
not replace app access, organization membership, or action permissions.

## Hosting

Shared SaaS is the V1 deployment direction. A later Kanche-managed dedicated
offering would use the same identity service with deployment assignment separate
from tenant identity. Define and test issuer routing, key, and session migration
before offering moves from shared hosting. Customer-owned cloud hosting is a
separate offering to evaluate, not an approved implementation.
