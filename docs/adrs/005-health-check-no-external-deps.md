# ADR-005: Health Checks Without External HTTP Dependencies

**Status**: Accepted
**Date**: 2026-04-02

## Context

Health checks need to make HTTP GET requests to verify service readiness (e.g., `http://127.0.0.1:8090/v1/health`). Argonaut is written in Cyrius — there is no external HTTP client library available. However, the core reasons this constraint is a feature (not a limitation) are:

- Every dependency is attack surface. HTTP client libraries pull in TLS stacks, compression, and connection pools.
- Init systems boot before the network stack is fully up. TLS certificate verification may fail during early boot.
- Health checks only target localhost services — TLS is unnecessary.
- Dependency count directly impacts audit burden and supply chain risk.

## Decision

HTTP health checks use raw TCP sockets with hand-written HTTP/1.1 request/response parsing.

- Connect via raw TCP socket syscall with timeout.
- Send `GET /path HTTP/1.1\r\nHost: host\r\nConnection: close\r\n\r\n`.
- Read the status line, parse the 3-digit status code, check for 2xx range.
- Body is not read — health endpoints return status in the HTTP status code.

## Consequences

- **Positive**: Zero additional dependencies for HTTP health checks. No TLS stack, no HTTP parser library.
- **Positive**: Minimal code surface to audit (~40 lines).
- **Positive**: Works during early boot before TLS certificates are available.
- **Negative**: No HTTPS support. Health checks must use plaintext HTTP. This is acceptable because checks target localhost only.
- **Negative**: No HTTP/2, no chunked encoding, no redirects. Health endpoints must respond with a simple status code on the first request.
- **Negative**: No connection reuse. Each check opens a new TCP connection. Acceptable for periodic checks (every 10-15 seconds).

## Alternatives Considered

- **HTTP client library**: Not applicable in Cyrius — no external library registry. Even if available, any HTTP client would pull TLS, compression, connection pool dependencies. Rejected.
- **`curl` via command**: Shell out to `curl`. Works but adds process spawn overhead per check and reintroduces shell injection risk.
- **TCP-only (no HTTP parsing)**: Even simpler — just check if the port accepts connections. Rejected because many services are TCP-reachable before they are ready to handle requests.
