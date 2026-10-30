# Architecture Decision Records

Decisions about argonaut: what was chosen, the context, and the consequences accepted. Read
these when you would reasonably ask *"why does it work this way?"*

## Conventions

- **Filename:** `NNN-kebab-case-title.md`, three digits. Never renumber.
- **One decision per ADR.** A decision that replaces an earlier one is a new ADR, and the
  earlier one's status becomes `Superseded by ADR-NNN`.
- **Status lifecycle:** `Proposed` → `Accepted` → (optionally) `Superseded` or `Deprecated`.
- Start from [`000-template.md`](000-template.md).

## ADR vs. architecture note vs. guide

| Kind | Lives in | Answers |
|---|---|---|
| ADR | `docs/adrs/` | *Why did we choose X over Y?* |
| Architecture note | [`docs/architecture/`](../architecture/) | *What non-obvious constraint is true about the code?* |
| Guide | [`docs/guides/`](../guides/) | *How do I do X?* |

## Index

| ADR | Decision | Status |
|---|---|---|
| [001](001-language-choice.md) | Rust as the implementation language | Superseded: ported to Cyrius |
| [002](002-safe-command-abstraction.md) | `SafeCommand`: explicit argv, never a shell string | Accepted |
| [003](003-library-plus-binary-architecture.md) | A library plus a separate PID-1 binary | Accepted, amended |
| [004](004-edge-boot-security-model.md) | Edge boot: dm-verity, LUKS and a read-only rootfs | Accepted |
| [005](005-health-check-no-external-deps.md) | Health checks with no external HTTP dependency | Accepted |
| [006](006-pid1-binary-separation.md) | PID 1 separated from the service manager | Accepted, amended |
| [007](007-feature-gated-ecosystem-deps.md) | Feature-gated AGNOS ecosystem dependencies | Superseded: Cyrius has no feature gates |
| [008](008-wave-based-parallel-startup.md) | Services start in parallel waves by dependency depth | Accepted |
| [009](009-service-type-dispatch.md) | Service types: simple, forking, oneshot | Accepted |
| [010](010-prlimit-over-preexec.md) | Resource limits through `prlimit(1)` | Superseded by ADR-014 |
| [011](011-per-service-security-model.md) | Per-service seccomp, Landlock and capabilities | Superseded by ADR-013 |
| [012](012-api-stability-pre-1.0.md) | API stability before 1.0 | Superseded: past 1.0 |
| [013](013-policy-enforcement-in-the-consumer.md) | Service policy is enforced by the consumer, through a pre-exec hook | Accepted, narrowed by ADR-014 |
| [014](014-spawn-applies-limits-env-files-and-names.md) | argonaut applies limits and env files at spawn, and checks names at registration | Accepted |
