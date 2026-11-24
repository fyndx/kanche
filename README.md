# Kanche

Kanche is a Rust identity service being built for straightforward developer
integration and explicit tenant and application boundaries. The current service
provides the health foundation and the first app-owned password-login slice with
PostgreSQL, a TypeScript server SDK and Next.js adapter. See the
[working integration/setup contract](docs/contracts/app-owned-integration.md).
The [hosted login profile](docs/contracts/hosted-integration.md) uses the same
app-bound sessions. Organizations, signup/recovery and console APIs remain undelivered.
[Both-profile verification evidence](docs/verification/hosted-login.md) records
fresh-checkout results, browser findings and remaining operational limits.

- [Product features and flows](Features.md)
- [Roadmap](docs/architecture/v1-roadmap.md)
- [Product direction](docs/architecture/0002-product-direction.md)
- [Framework decision](docs/architecture/0001-axum.md)
- [Engineering standards](AGENTS.md)

## Development setup

The supported verification environment is Linux x86_64 (CI: Ubuntu 24.04),
with Git, GNU Make, Python 3.12 or newer, rustup, and Node.js **26.7.0** with
npm **11.19.0**, plus Docker for disposable PostgreSQL integration tests. Rust **1.99.1**, Rustfmt, and Clippy are pinned in
`rust-toolchain.toml` and installed by rustup on first use. The security binary
installer supports Linux x86_64; other platforms have not been verified.

From a fresh checkout, with the prerequisites installed:

```sh
# If using nvm:
nvm install
nvm use
npm install --global npm@11.19.0
make setup
make check
make security
make dev
```

`make setup` installs locked repository tooling and fetches locked Rust
dependencies. Installation and security checks require network access; running
the health service requires no account or credentials. Integration tests use synthetic identities and ephemeral test keys. `make check` runs
formatting, Clippy, JavaScript/documentation checks, Rust tests, binary lifecycle
checks, and Rust documentation. `make security` scans files for secrets with
redacted findings and checks dependency advisories and licenses. CI also scans
Git history. Scanning tools are checksum-verified and installed into ignored
`.tools/`. Use `make help` for individual commands and `make fmt` to format changes.

See [contribution instructions](CONTRIBUTING.md), [security policy](SECURITY.md),
and [repository administration](docs/repository-administration.md). The remote is
`git@github.com:fyndx/kanche.git`; `@chakrihacker` is the code owner. Packages remain
unpublished. Hosted repository settings require separate verification.

## Server configuration and operation

The service reads process environment only. `.env.example` documents defaults
and is not loaded automatically. Explicit values override defaults; invalid
values fail startup without echoing their contents.

| Variable               | Default                        | Accepted values                                                                                                         |
| ---------------------- | ------------------------------ | ----------------------------------------------------------------------------------------------------------------------- |
| `KANCHE_BIND_ADDR`     | `127.0.0.1:8080`               | IP address and port, including bracketed IPv6; port 0 chooses an ephemeral port                                         |
| `KANCHE_DATABASE_URL`  | unset (health only)            | PostgreSQL URL with explicit loopback IP; run operator migration first; never log the value                             |
| `KANCHE_HOSTED_ORIGIN` | unset (hosted routes disabled) | Canonical HTTPS origin or loopback HTTP for development; see [hosted integration](docs/contracts/hosted-integration.md) |
| `KANCHE_LOG_LEVEL`     | `info`                         | `error`, `warn`, `info`, `debug`, `trace`                                                                               |

Keep loopback binding for local use. Binding an external interface requires an
explicit configuration decision. `RUST_LOG` is not consumed. JSON logs record
startup address, generated request ID, matched route template, response status,
and handler latency. Raw URLs, query strings, headers, bodies, and supplied
configuration values are omitted. Every response gets a fresh `x-request-id`;
incoming IDs are ignored. Request logs describe response creation rather than
streamed body completion.

Send SIGINT (Ctrl+C) or SIGTERM to stop accepting connections and drain active
requests. Shutdown exits unsuccessfully after ten seconds if draining stalls.
Bind or startup failures exit unsuccessfully with an actionable message. The
lifecycle check covers both signals and port reuse; the forced deadline path
has not yet been exercised with an intentionally stalled handler.

## Run the server

Run `make dev` (or `cargo run --locked`) and request
`http://127.0.0.1:8080/healthz`. The response is `ok`.

## License

Kanche's original source code and project files are licensed under the
[Mozilla Public License 2.0](LICENSE). Source files carry SPDX identifiers;
the root license applies to original project files that cannot carry comments.
Third-party dependencies retain their own licenses and notices.
