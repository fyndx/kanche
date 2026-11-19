# Developer console and onboarding

**Status:** Planned, Tasks 14–15. No developer console or workspace APIs exist.
Resource relationships follow the [SaaS proposal](../architecture/saas-plan.md)
subject to the applicable design reviews.

## Workspace and console access

Task 14 establishes developer accounts, workspace membership, and console
permissions. The proposal allows a developer account to belong to multiple
workspaces. Workspace admins manage their own tenants and apps and must be
denied access to another workspace's resources.

Console permissions and end-user app access are checked separately. Being an
organization administrator in an app does not imply workspace administration.
Exact developer authentication, permission roles, and membership APIs are still
to be specified.

## Self-service provisioning

Task 15 delivers workspace signup, multiple tenant creation, and two app setup
paths:

1. **Create an app with its own directory:** provision a tenant and its first app.
2. **Add an app to an existing directory:** register an app in a selected tenant.

Both paths use the same resource model. A developer starting with one app should
reach a working integration without first choosing directory-sharing policies.
Verify that retries produce one usable provisioning result. Define authorization,
migrations, partial-failure handling, and recovery as part of delivery. The
initial operator/fixture path in [Task 03](identity.md) precedes self-service.
