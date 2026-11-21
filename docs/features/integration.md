# Integration and developer experience

## Scope and status

This product documentation describes implemented behavior and proposed work;
it does not approve a feature or replace a reviewed contract. **Implemented**
means verified repository behavior. **Planned** means unshipped delivery work.
**Proposed** needs a design decision. **Deferred** is outside V1.

The service implements the first app-owned provisioned-password login and online
session slice. See the [concrete integration guide](../contracts/app-owned-integration.md).
Hosted login is required next; organizations and console remain unimplemented. See the
[product direction](../architecture/0002-product-direction.md),
[design gates](../architecture/v1-roadmap.md), and [feature index](../../Features.md).

## First integration contract

**Status:** App-owned prerequisites completed under delegated approved defaults;
[KN-2](https://fyndx.atlassian.net/browse/KN-2) retains the hosted-profile follow-up.

Define Kanche's native developer configuration, browser and server
responsibilities, origins, redirects, errors, and supported integration path
before implementing routes. The intended flow is configuration, sign-in,
a protected request, renewal, and sign-out. The app-owned API/SDK and session behavior are recorded in the
[design gate](../architecture/0003-app-owned-login.md).

Keep design discussions and decisions in
[Confluence](https://fyndx.atlassian.net/wiki/spaces/KA) and implementation scope,
acceptance criteria, and validation evidence in
[Jira](https://fyndx.atlassian.net/issues/?jql=project%20%3D%20KN).

## Developer workflow

**Status:** Repository setup and checks available.

The [README](../../README.md#development-setup) records pinned prerequisites,
configuration, and the setup/check/run path. The
[contribution guide](../../CONTRIBUTING.md) covers development checks. Validate
the ordinary developer entry point and relevant failure paths for each slice;
keep examples and migration guidance aligned with shipped behavior.
