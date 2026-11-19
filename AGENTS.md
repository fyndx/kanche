# Kanche engineering standards

Apply these rules to every design, implementation, review, and release.

1. **Build for enterprise reliability and security.** Prefer explicit contracts,
   simple maintainable designs, secure defaults, and predictable failure behavior.
   Validate untrusted input and enforce tenant, application, and authorization
   boundaries. Prove relevant invariants with repeatable tests covering failure,
   expiry, replay, concurrency, and recovery as the feature requires. Treat
   compatibility, migrations, observability, and operational recovery as part of
   delivery. Claim stability or compatibility only to the extent supported by
   recorded evidence; operational maturity must be demonstrated.
2. **Make developer experience a product requirement.** Provide consistent APIs,
   actionable errors that do not expose secrets, safe configuration, and a short,
   documented path from a fresh checkout to a working integration. Keep examples,
   pinned fixtures, and documentation aligned with shipped behavior. Prefer
   backwards-compatible changes; explain breaking changes and their migration
   path. Verify the normal developer workflow as well as internal behavior.

## Planning and design gates

- Use the Kanche Confluence space for planning/design discussions and Jira
  project KN for delivery tasks. See `docs/architecture/v1-roadmap.md` for links.
- Review the native integration contract before authentication routes, the
  session lifecycle before session routes, and ownership/access before
  organization implementation. Migration of a task is not design approval.
- Distinguish observed behavior, test evidence, proposals, and assumptions.
  Record evidence gaps and remaining operational limits.

## Delivery practices

- Use synthetic identities and ephemeral test keys. Never commit credentials,
  raw tokens, session cookie values, or raw network captures.
- Follow `docs/architecture/v1-roadmap.md`. Complete the
  applicable design gate before implementing authentication or session routes.
- Run the checks appropriate to the change and record their results and limits.
  Passing checks do not establish production readiness.
