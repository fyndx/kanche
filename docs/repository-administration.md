# Repository administration

The local repository uses `main` and the user-selected SSH remote:

```text
origin  git@github.com:fyndx/kanche.git
```

The maintainer and code owner is `@chakrihacker`, recorded in
[CODEOWNERS](../.github/CODEOWNERS). Ensure that account has write or admin
access to the hosted repository. Security reports go to `security@fyndx.io`
(the domain's catch-all routes to the maintainer), with `hello@fyndx.io` as a
fallback. See [SECURITY.md](../SECURITY.md).

Local configuration does not publish code or verify hosted permissions, branch
rules, mail delivery, or GitHub security settings. Publishing changes requires
a separate commit and push.

## Before hosting or accepting external contributions

- Kanche uses [MPL-2.0](../LICENSE), declared in Cargo and the npm tooling package.
  Keep source SPDX identifiers and third-party notices intact. Packages remain
  private/unpublished until a separate release decision.
- Review the initial file list before committing.
  Run `make check` and `make security`, including the untracked-file secret scan.
- Verify delivery to the published security contact. Enable GitHub private
  vulnerability reporting when available.

## GitHub settings to configure

Configure these in [fyndx/kanche settings](https://github.com/fyndx/kanche/settings).
After the initial push and successful workflow runs, select the emitted job
names as required checks. Hosted enablement has not been verified.

| Location                             | Setting                                                                                                                                                                                                                                                                                                                                                  |
| ------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| General                              | Default branch `main`; allow squash merging, disable merge commits and rebase merging, and automatically delete merged head branches.                                                                                                                                                                                                                    |
| Rules → Rulesets                     | Create an **Active** branch ruleset targeting `main`. Require pull requests, checks `quality` and `security` from GitHub Actions, branches up to date before merging, resolved review conversations, linear history, and protection against deletion and force pushes. Do not restrict all updates or branch creation. Avoid routine bypass permissions. |
| Rules → Rulesets, while working solo | Leave required approvals and required code-owner approval disabled. A PR author cannot approve their own PR. CODEOWNERS still identifies the maintainer for other contributors' PRs. When a second reviewer joins, require one approval, code-owner review, and dismissal of stale approvals.                                                            |
| Actions → General                    | Allow GitHub-authored actions used by these workflows; require full commit SHA pins if the option is available. Use read-only workflow token permissions. Leave Actions creating/approving PRs disabled. Require approval for all outside collaborators' workflow runs; do not give fork workflows write tokens or secrets.                              |
| Advanced Security                    | Enable the dependency graph, Dependabot alerts and security updates, secret scanning and push protection where available. Dependabot version-update schedules already live in `.github/dependabot.yml`. Tool binary pins need manual review.                                                                                                             |
| Advanced Security, public repository | Enable private vulnerability reporting. Email in SECURITY.md remains available if this feature is unavailable.                                                                                                                                                                                                                                           |
| Notifications                        | Watch security alerts and workflow failures, and verify delivery to the maintainer. Check that the scheduled security workflow runs.                                                                                                                                                                                                                     |

Rulesets and security features depend on repository visibility and the GitHub
plan. Public repositories support private vulnerability reporting; verify the
settings available for this repository rather than assuming all features apply.
See GitHub's [ruleset reference](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/available-rules-for-rulesets),
[Actions settings](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/enabling-features-for-your-repository/managing-github-actions-settings-for-a-repository),
and [private reporting setup](https://docs.github.com/en/code-security/how-tos/report-and-fix-vulnerabilities/configure-vulnerability-reporting/configure-for-a-repository).

These settings cannot be enforced by adding files to a local checkout. Confirm
them on the hosting service; this document is not evidence that they are enabled.

## Deferred until the relevant roadmap task

Database selection, migrations, deployment images and infrastructure, production
secrets, release signing and provenance, backup/restore drills, and release
automation will be delivered with their approved feature or release scope.
No authentication or session design gate is satisfied by repository boilerplate.
