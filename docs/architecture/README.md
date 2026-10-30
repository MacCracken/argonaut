# Architecture notes

Non-obvious constraints, quirks and invariants that a reader cannot derive from the code
alone. Numbered in order of discovery. Never renumber, and never reuse a number.

Not decisions (those are [ADRs](../adrs/)) and not how-tos (those are
[guides](../guides/)). A note describes *how the world is*, not *what we chose*.

A toolchain pin bump or a dependency refresh does not earn a note: the CHANGELOG entry is
the record (CLAUDE.md, Docs).

## Items

| Note | What it says | What it affects |
|---|---|---|
| [001](001-cross-arch-aarch64.md) | aarch64 portability is in the source, not the compiler; CI requires the native and cross builds to be byte-identical | Any syscall in `src/`; the aarch64 release artifact |
| [002](002-qemu-pid1-harness.md) | The qemu PID-1 harness needs KVM and an invariant TSC | `qemu/`; the CI `qemu-pid1` job |
| [005](005-edge-boot-tpm-key-policy.md) | The TPM key-release policy belongs to the enroller, not argonaut | Edge boot; kybernet's LUKS enrolment |

Numbers 003 and 004 were toolchain notes, deleted at 1.8.5. They are not reused.

## Reference

Unnumbered, kept current rather than appended to:

- [`overview.md`](overview.md): the system context and module map.
- [`feature-gaps.md`](feature-gaps.md): what production init systems provide, and where
  each item stands in argonaut.
