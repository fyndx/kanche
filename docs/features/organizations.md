# Organizations and application authorization

**Status:** Proposed model; Tasks 11–13 are planned. No organization routes,
tables, or claims are approved by this page. Reopen the
[SaaS architecture proposal](../architecture/saas-plan.md) at Task 11.

## Organization design gate

Task 11 compares app-owned organizations with tenant-owned organizations and
explicit app grants. Decide ownership, administrative authority, membership,
roles, and active context before implementation. Walk through one customer
using two products, independent product admins, an app invitation, and member
removal. Split unresolved decisions into separate design tasks when necessary.

## Membership and invitations

Task 12 implements the approved organization lifecycle, invitations, acceptance,
and membership management. Verify normal acceptance, expiry, duplicate handling,
and removal. Invitation targeting, role assignment, retry rules, and removal
semantics remain to be decided. Console workspace membership is separate from
end-user organization membership.

## App grants and context

Task 13 implements approved access rules and authorization checks. The proposal
requires an app grant for the organization, active user membership, and the
permissions needed for an action. It also illustrates user-level app access;
the exact grant subjects and relationships require Task 11 approval.

The acceptance flow uses one organization across two apps. A grant change affects
its app scope; membership removal affects access through that organization
across apps. Define the effect on existing sessions and tokens. Decide whether
active organization belongs in a session or request context and how context
switching is validated; neither representation is selected yet.

## Roles and permissions

Define who can administer organizations, invitations, memberships, and grants.
The proposal allows different permissions per app, such as an approver in one
product without the same authority in another. Role definitions, delegation,
and enforcement points require review; role enforcement or context switching
may become separate tasks. Product entitlements are a
[separate deferred concern](future.md#billing-and-entitlements).
