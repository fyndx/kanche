# Kanche SaaS architecture plan

**Status:** Ready for review

The [product direction decision](0002-product-direction.md) establishes a native
Kanche contract. This plan
remains a proposal; organization ownership and access still require Task 11 review.

Kanche will use tenant-scoped identity and app-scoped access. A developer can
create multiple tenants under one workspace. Each tenant starts with one
managed sign-in connection and can serve one app or a suite of apps. A
standalone app can start in its own tenant and share that directory with
additional apps later.

## Resource model

| Resource        | Responsibility                                                                                                                                                                                          |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Workspace       | Owns tenants, console membership, administrative permissions, and the bill the customer pays Kanche. A developer account can belong to multiple workspaces.                                             |
| Identity tenant | Owns end-user accounts, organizations, and sign-in connections. Apps in the same tenant can share accounts. Use separate tenants for independent account lifecycles and for development and production. |
| Application     | Registers a sign-in client and defines who can access it. Web and mobile clients can be registered separately and use the same tenant.                                                                  |
| Organization    | Represents a business using the customer's product. The tenant owns the organization, and the organization has its own memberships. An organization can use multiple apps through explicit grants.      |
| Connection      | Defines how users authenticate and which identifiers the sign-in flow accepts. Credentials and login identifiers resolve to a stable user ID within the tenant.                                         |
| Product         | Defines what the customer sells. Multiple app clients can serve one product, and a subscription can cover multiple products.                                                                            |

Console membership and end-user organization membership are separate.
An Acme workspace administrator can provision tenants and apps. A Northwind
organization administrator can manage Northwind's members within their
delegated permissions.

The console should offer two paths. **Create an app with its own directory**
creates a tenant and its first app. **Add an app to an existing directory**
registers a client in a selected tenant. Use one resource model for both
paths. Customers who only need one app can complete setup without making
directory-sharing decisions.

## Access and account lifecycle

A user account, an organization membership, and an app grant have distinct
lifecycles. A B2B app requires explicit access for the organization, an active
membership for the user, and the permissions required by the action. Issue and
renew app sessions in the applicable app context. Recheck app access at renewal
and define how already-issued sessions and tokens expire or are revoked.

For example, Acme can grant Northwind access to Expenses and Vendors while
giving Maya approver permissions only in Expenses. Revoking Maya's Expenses
grant affects that app. Removing her Northwind membership affects access
through Northwind across both apps. Disabling her tenant account affects all
apps in that tenant. An internal employee portal with independently managed
accounts gets its own workforce tenant under Acme's workspace.

## Connections and credential stores

**Implement a connection now.** Tenant provisioning creates one managed
password connection, and sign-up and sign-in use it. Give the connection a
stable ID. Scope login identifiers to the connection, and associate credentials
and identifiers with the tenant user. The initial flow can require email while
the underlying user record allows an account without one. Memberships, app
grants, and billing reference stable user or organization IDs.

Add connection configuration when introducing a second sign-in method that
requires it. For workforce SSO in a shared tenant, require both the workforce
connection and an employee grant for the portal. For student ID login, scope
the identifier to its issuing school and define recovery before enabling the
flow. A student ID identifies the account, and the user proves ownership
through a password or the school's identity provider. Require verified account
linking when identities from different sources belong to the same person.

**Defer multiple database connections.** Start with one managed password
directory per tenant. Use grants for app access, separate tenants for
independent account lifecycles, and SSO connections for external identity
providers. Revisit multiple password stores when a customer needs one app to
accept credentials from two existing stores that must stay live. That use
case warrants explicit rules for store selection, account linking, and recovery.

## Billing and entitlements

Separate **platform billing** (what the workspace pays Kanche) from **product
billing** (what an end user or organization pays the workspace). For product
billing, a subscription is attached to a user for B2C or an organization for
B2B, and grants entitlements to the products or features it covers.

For example, Northwind can buy Expenses once and use it through both the web
and mobile clients. It can buy Vendors separately or choose a bundle. A user
approving expenses still needs app access, Northwind membership, the approver
permission, and the relevant paid entitlement. The subscription's end-of-access
policy determines when the entitlement expires. Account access and membership
follow their own lifecycles.

Define the payer and entitlement boundaries now. Introduce the product catalog,
payment integration, checkout, and subscription lifecycle as part of product
billing delivery.

## Hosting

Start with shared SaaS and use the same identity service for a later
Kanche-managed dedicated offering. Keep the deployment assignment separate
from the tenant ID. Sign-in and session refresh should work while the console
is unavailable. Dedicated hosting changes where a tenant runs and which
infrastructure it shares. Customer-owned cloud hosting is a separate offering
to evaluate later.

Use Kanche's native model for shared directories, cross-app organization
membership, and app grants. Start verification with a single-app tenant, then
prove shared-tenant access and mutation scopes. Membership removal and grant
changes must affect the intended resources and sessions only.

## Delivery sequence

1. **Identity foundation.** Build tenants, app registration, and the built-in
   connection into the first sign-up, sign-in, and session flows. Verify that
   two tenants reject each other's credentials and sessions, and run the
   ordinary single-app integration path.
2. **Shared SaaS and B2B access.** Add workspace signup and self-service tenant
   and app provisioning. Add organizations, invitations, and app grants. Verify
   that one organization can use two apps with different permissions, and
   that app revocation and organization removal affect the intended scope.
3. **Expanded sign-in and product billing.** Add configurable connections and
   alternative identifiers as the supported flows require. Deliver product
   billing around user and organization payers. Verify that two clients can
   share one entitlement and that cancellation follows the configured policy.
4. **Dedicated hosting.** Provision a customer deployment using the same
   service. Define and test migration of issuer routing, keys, and sessions
   before offering moves from shared hosting.
