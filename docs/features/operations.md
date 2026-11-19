# Service and operations

## Current service

**Status:** Implemented service foundation, not production identity readiness.

`GET /healthz` returns `ok`. No authentication or session endpoints exist. See the
[README](../../README.md#run-the-server) for running the service and
[source](../../src/lib.rs) for the current router.

## Configuration

**Status:** Implemented.

The service reads process environment only; environment files are not loaded
automatically. `KANCHE_BIND_ADDR` defaults to `127.0.0.1:8080` and accepts an IP
address and port. `KANCHE_LOG_LEVEL` defaults to `info` and accepts `error`,
`warn`, `info`, `debug`, or `trace`. Invalid values fail startup without echoing
their contents. `RUST_LOG` is not consumed. See the
[configuration reference](../../README.md#server-configuration-and-operation).

## Observability

**Status:** Implemented request logging.

JSON logs record startup address, generated request ID, matched route template,
response status, and handler latency. Raw URLs, query strings, headers, bodies,
and supplied configuration values are omitted. Every response receives a fresh
`x-request-id`; incoming request IDs are ignored. Latency describes response
creation rather than streamed body completion. A customer-facing audit log or
log dashboard is not implemented.

## Shutdown and restart

**Status:** Implemented, with recorded test limits.

SIGINT and SIGTERM stop accepting connections and drain active requests. A
10-second drain deadline exits unsuccessfully if draining stalls. Startup and
bind failures exit unsuccessfully with actionable errors. The lifecycle smoke
check covers both signals and restarting on the same port; an intentionally
stalled handler has not exercised the forced-deadline path. Existing service
checks live in [tests](../../tests/service.rs) and the
[lifecycle script](../../scripts/smoke-server.py).

## Shared SaaS release readiness

**Status:** Planned, Task 16; earlier slices do not establish release readiness.

Verify complete developer and end-user flows under deployment conditions. Record
tenant isolation evidence, the supported integration path, authentication and
renewal during console downtime, migration procedures, and recovery procedures.
Break operational gaps into scoped tasks. Require repeatable failure, expiry,
replay, concurrency, and recovery evidence where relevant to the delivered slice.

Use the [repository checks](../../CONTRIBUTING.md) and
[security policy](../../SECURITY.md). Passing checks does not prove
production security or operational recovery. Dedicated hosting
and deployment migration are [deferred](future.md#hosting).
