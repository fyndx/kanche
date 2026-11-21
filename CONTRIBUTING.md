# Contributing to Kanche

Read [AGENTS.md](AGENTS.md) and the [planning and design gates](docs/architecture/v1-roadmap.md)
before changing behavior. Work on one reviewable scope. Authentication, session,
and organization changes require their applicable design reviews.

## Development

Follow the [fresh checkout setup](README.md#development-setup), then run:

```sh
make fmt
make check # includes disposable PostgreSQL; requires Docker
make security
```

CI runs the quality and security checks with pinned tools on Ubuntu 24.04.
JavaScript linting uses Oxlint; JavaScript, configuration, and documentation
formatting uses Oxfmt. Run `npm run lint`, `npm run fmt`, or `npm run fmt:check`
for these checks independently. Rust formatting continues to use Rustfmt.
Add tests for changed contracts and relevant security or failure behavior.
Do not add tests merely to mirror simple edits. Use temporary keys, synthetic
identities, and isolated resources. Never record tokens, cookies, credentials,
raw captures, or personal data in Git or logs.

## Planning and review

Use the [Kanche Confluence space](https://fyndx.atlassian.net/wiki/spaces/KA)
for planning and design discussions, and the
[Kanche Jira project](https://fyndx.atlassian.net/issues/?jql=project%20%3D%20KN)
for scoped delivery tasks. Search existing pages and issues before creating new
ones. Record design approvals and evidence limits; migration alone does not
approve a design or mark implementation complete.

Use the PR template to explain the problem, resulting behavior, evidence, and
limits. Include configuration, compatibility, migration, and recovery notes as
applicable. Use the [ADR template](docs/architecture/template.md) for decisions.
Keep observed behavior, test evidence, proposals, and assumptions distinct.

## Tooling and dependency maintenance

Review automated dependency PRs and lockfile changes. Pin GitHub Actions to full
commit SHAs and keep workflow permissions minimal. Update Node/npm versions
in `.nvmrc`, `package.json`, `package-lock.json`, and CI together. Update Rust in
`rust-toolchain.toml` and `Cargo.toml`, then verify the fresh-checkout workflow.

Run `make security` before committing. Its directory scan covers untracked files;
CI also scans Git history. Scanner output is redacted. Rust license checks cover
the project and dependencies; npm checks the root declaration and all locked
dependencies, including development packages. These are policy checks, not legal
advice. Review new scanner release versions and SHA-256 hashes together.

Do not add blanket exclusions or audit bypasses. For an exception, record the
advisory or package, exact scope, reason, owner, expiry date, and compensating
controls in a reviewed change. Remove or review expired exceptions.

Ownership and hosting requirements are recorded in
[repository administration](docs/repository-administration.md). Contributions to
Kanche's original code are made under [MPL-2.0](LICENSE). Add an
`SPDX-License-Identifier: MPL-2.0` comment to new source files and preserve
third-party license notices when incorporating separately licensed work.
