# Repository foundation verification

This record covers the health endpoint, configuration validation, structured
request logging, generated request IDs, graceful shutdown, and repository
quality checks. Authentication, sessions, and organization APIs remain subject
to the [planning and design gates](../architecture/v1-roadmap.md).

## Verification contract

The verified platform is Linux x86_64. CI targets Ubuntu 24.04. Tool versions and
setup requirements are defined in the [development guide](../../README.md#development-setup)
and repository manifests.

| Command         | Coverage                                                                                                         |
| --------------- | ---------------------------------------------------------------------------------------------------------------- |
| `make setup`    | Pinned tool versions, locked npm installation, and locked Rust dependency fetch.                                 |
| `make check`    | Formatting, Clippy, JavaScript lint, documentation links, Rust tests, process lifecycle, and Rust documentation. |
| `make security` | Secret scanning, dependency advisories, source policies, and dependency licenses.                                |

## Recorded evidence

### Foundation validation · 2026-11-19

`make setup`, `make check`, and `make security` passed against a clean source
copy using the pinned toolchain. Dependency caches were available; this does
not demonstrate installation with empty caches. Rust documentation passed with
warnings denied.

The two Rust integration tests and process lifecycle checks passed. Coverage
includes invalid configuration, occupied-port failure, log redaction, request
ID correlation, SIGINT/SIGTERM shutdown, and port reuse. Formatting, JavaScript
lint, and relative documentation link checks also passed.

Secret scanning reported no findings. Dependency advisory, source, and license
checks passed. npm reported no known advisories at the time of validation.
Rust policy checks reported non-failing warnings for duplicate `syn` versions
and unused license allowances.

### Oxlint and Oxfmt migration · 2026-11-19

Oxlint 1.94.0 and Oxfmt 0.79.0 replace ESLint and Prettier. The npm and Make
commands retain their existing entry points. Oxfmt preserves the 80-column
width and generated-file and lockfile exclusions, adds TOML formatting, and
sorts `package.json` keys.

The lint configuration carries over 62 rules at error severity, including
Oxlint's experimental `no-undef` and `no-useless-assignment` rules. There are
no equivalent rules for `no-dupe-args` and `no-octal`. Ad hoc ES module checks
confirmed parser rejection of duplicate arguments and legacy octal literals,
acceptance of Node globals, and rejection of undefined names and unused
variables. These checks were not retained as regression tests and do not
establish full ESLint equivalence.

A locked dependency reinstall with `npm ci --offline`, `make check`, and
`make security` passed. Both Rust integration tests, process lifecycle checks,
formatting, lint, and documentation link checks passed. Security results and
non-failing Rust policy warnings were consistent with foundation validation.

## Evidence limits

- Results describe the recorded revisions and validation dates. They do not
  establish production readiness or current dependency security status.
- Hosted workflow execution, repository protections, private reporting settings,
  and email delivery were not verified. CI additionally scans Git history.
- The forced shutdown deadline has not been exercised with a stalled handler.
- Request logs cover response creation rather than streamed body completion.
- Documentation checks cover relative file targets; remote URLs and heading
  anchors are not checked.
- Dependency policy permits duplicate crate versions with warnings. npm blocks
  high and critical advisories; lower-severity findings require review. License
  checks do not replace distribution review.
- The tooling migration was verified on Linux x86_64. Other operating systems
  and comparative performance were not evaluated.
