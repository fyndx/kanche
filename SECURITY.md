# Security policy

Kanche is in development. No version currently has a production security or
long-term support commitment. Passing tests do not establish production
readiness.

## Reporting a vulnerability

Do not create a public issue containing vulnerability details, credentials,
tokens, session cookies, private keys, raw captures, or personal data.

Email [security@fyndx.io](mailto:security@fyndx.io), or use
[hello@fyndx.io](mailto:hello@fyndx.io) as a fallback. The maintainer has confirmed
that domain aliases route to their inbox. Include the affected revision, impact,
and a minimal reproduction with synthetic identities. Share sanitized
observations only. Email delivery has not been independently tested.

Once GitHub private vulnerability reporting is enabled for
[fyndx/kanche](https://github.com/fyndx/kanche), you can also use
**Security → Advisories → Report a vulnerability**. Its enablement has not yet
been verified. No response-time commitment is defined yet. See
[administration setup](docs/repository-administration.md).

## Development safeguards

Run `make security` for secret scanning, Rust advisories and dependency policies,
npm advisories, and npm dependency license checks. Scheduled CI reruns these
checks because new advisories can affect unchanged lockfiles. Checks require
access to GitHub, package registries, and the RustSec advisory database.

Never upload raw logs or scanner artifacts containing secrets. If a credential
is exposed, revoke or rotate it first; deleting a file or rewriting history is
not a substitute. Coordinate any history rewrite with maintainers.
