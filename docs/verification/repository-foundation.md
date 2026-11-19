# Repository foundation verification

The service foundation includes the health endpoint, configuration validation,
safe structured request logging, generated request IDs, and bounded graceful
shutdown. Authentication and session routes are not implemented. Detailed
planning and source material are maintained in the
[Confluence planning archive](https://fyndx.atlassian.net/wiki/spaces/KA/pages/97583105),
with delivery tasks in [Jira KN](https://fyndx.atlassian.net/issues/?jql=project%20%3D%20KN).

## Current verification scope

The supported verification target is Linux x86_64 (CI: Ubuntu 24.04), with Rust
1.99.1, Node.js 26.7.0, npm 11.19.0, and Python 3.12 or newer.

| Command         | Coverage                                                                                                                                                       |
| --------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `make setup`    | Verify pinned tools; install locked npm tooling; fetch locked Rust dependencies.                                                                               |
| `make check`    | Rustfmt, Clippy with warnings denied, Prettier, ESLint, relative documentation links, Rust integration tests, binary lifecycle checks, and Rust documentation. |
| `make security` | Secret scanning, Rust advisory/license/source policies, npm advisories, and npm license policy. CI also scans Git history.                                     |

## Validation results · 2026-11-19

`make setup`, `make check`, and `make security` passed in a clean temporary
source copy without local environment files, installed dependency trees, build
products, or scanner binaries. Installed runtimes and global package caches
were available. Rust documentation was built with warnings denied, matching CI.

Two Rust integration tests and the binary lifecycle checks passed. Formatting,
JavaScript lint, and relative documentation links passed. Secret scanning found
no findings. Rust advisory/license/source checks passed, with non-failing
warnings for duplicate `syn` versions and unused license allowances. npm reported
zero known advisories, and its license policy passed.

Planning was preserved in Confluence before repository cleanup: 15 document
pages were read back, the complete source attachment was downloaded and checked
against its archive hash and all 70 per-file hashes, and 16 migrated Jira tasks
were checked against their original scope and acceptance criteria. Archived
planning does not establish current implementation or approve a design gate.

## Limits

- Local commands do not verify hosted workflow execution, repository protections,
  private reporting settings, or email delivery.
- Binary checks cover invalid configuration, occupied-port failure, safe logs,
  request ID correlation, SIGINT/SIGTERM shutdown, and port reuse. The forced
  ten-second deadline has not been exercised with a deliberately stalled handler.
- Request logs cover response creation rather than streamed body completion.
- Documentation checks cover formatting and relative file targets; remote URLs
  and heading anchors are not checked.
- Dependency policy permits duplicate crate versions with warnings. License
  checks do not replace distribution review. npm blocks high/critical findings
  and prints lower-severity findings for review.
- Synthetic identities and temporary keys must be used for security tests.
  Repository checks do not establish production readiness.
