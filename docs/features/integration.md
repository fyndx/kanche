# Integration and developer experience

## Scope and status

This product documentation describes implemented behavior and proposed work;
it does not approve a feature or replace a reviewed contract. **Implemented**
means verified repository behavior. **Planned** means unshipped delivery work.
**Proposed** needs a design decision. **Deferred** is outside V1.

The service implements the first app-owned provisioned-password login and online
session slice. See the [concrete integration guide](../contracts/app-owned-integration.md).
The [hosted profile](../contracts/hosted-integration.md) adds a Kanche-owned password
page and a one-use server handoff with the same app-bound policy. Organizations
and console remain unimplemented. See the
[product direction](../architecture/0002-product-direction.md),
[design gates](../architecture/v1-roadmap.md), and [feature index](../../Features.md).

## First integration contract

**Status:** App-owned and hosted provisioned-password integrations are available
in this branch. See their public guides for configuration and limits.

Define Kanche's native developer configuration, browser and server
responsibilities, origins, redirects, errors, and supported integration path
before implementing routes. The intended flow is configuration, sign-in,
a protected request, renewal, and sign-out. The app-owned API/SDK and session behavior are recorded in the
[design gate](../architecture/0003-app-owned-login.md).

Detailed planning and approval history are maintained privately.

## Developer workflow

**Status:** Repository setup and checks available.

The [README](../../README.md#development-setup) records pinned prerequisites,
configuration, and the setup/check/run path. The
[contribution guide](../../CONTRIBUTING.md) covers development checks. Validate
the ordinary developer entry point and relevant failure paths for each slice;
keep examples and migration guidance aligned with shipped behavior.
