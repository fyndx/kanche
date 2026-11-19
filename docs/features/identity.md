# Identity foundation

**Status:** Planned, Task 03. Depends on the Task 02 integration contract.
The [SaaS architecture plan](../architecture/saas-plan.md) supplies the proposed
resource model; organization ownership remains subject to Task 11.

## Tenant and application provisioning

A workspace is proposed to own multiple identity tenants. A tenant owns its
account directory and registers applications. Apps within one tenant can share
accounts; separate tenants provide independent account lifecycles, including
separate development and production directories.

Task 03 starts with operator or fixture provisioning of a tenant, an application,
and its built-in password connection. Provisioning must be repeatable and include
migrations. Self-service console flows arrive in
[Tasks 14–15](console.md), rather than being prerequisites for the first sign-in.

## Managed password connection

Start with one managed password connection per tenant, with a stable connection
ID. Scope login identifiers to that connection and associate identifiers and
credentials with a stable tenant user ID. The initial flow can require email
while the user model allows an account without one. Memberships and grants refer
to stable IDs rather than mutable login identifiers.

Configurable connections and multiple password stores are
[deferred](future.md#connections-and-identifiers).

## Isolation

Task 03 must demonstrate two isolated tenants and two apps sharing one tenant.
Later sign-in and session slices must reject credentials and sessions used across
unauthorized tenant or app boundaries. Shared identity does not itself grant
application access. Exact authorization contracts are decided at the integration,
session, and organization gates; these are requirements to verify, not existing
guarantees.
