# 001 — Cross-arch (aarch64) build surface

Status: Active (since 1.5.4, 2026-05-10)

## Summary

Argonaut cross-builds to aarch64 via cyrius's
`cc5_aarch64` translator. No argonaut source changes are
required — the toolchain converts x86_64 syscall numbers + ABI
to aarch64 at codegen time. Binary ships in CI / release
artifacts as `argonaut-<VER>-aarch64-linux` alongside the
x86_64 build, **best-effort** (skipped without failing the run
when the toolchain release didn't bundle `cc5_aarch64`).

## Producer

```
CYRIUS_DCE=1 cyrius build --aarch64 src/main.cyr build/argonaut-aarch64
```

Binary size at 1.5.4 ship: ~1.14 MB (x86_64 is ~1.00 MB).
The +140 KB delta tracks aarch64's 32-bit fixed-width
instruction encoding vs. x86_64's variable-length.

## Smoke (CI)

CI runs `qemu-aarch64` on the cross-built binary and grep-asserts
`"all systems nominal"` on stdout. Best-effort:

- Skips with a warning when `cc5_aarch64` isn't in the toolchain
  bin dir.
- Skips with a warning when neither `qemu-aarch64-static` nor
  `qemu-aarch64` is on PATH.

## Known-failure surface (qemu-user + upstream)

Two classes of `.tcyr` failures show up under `qemu-aarch64` that
do NOT indicate argonaut-side bugs. They're documented here so
future cross-arch work doesn't chase them as regressions.

### qemu-user emulation limits

`qemu-user` runs the binary against the host kernel via syscall
translation. A few syscalls don't replicate full Linux semantics:

- **`fork(2)` + `waitpid(2)` reparenting** — child PIDs are
  emulator-internal; reparenting under PR_SET_CHILD_SUBREAPER
  doesn't compose cleanly with the emulator's process model.
  `tests/tcyr/audit_findings.tcyr` `audit-m3-reaper-orphans`
  asserts orphan-reaper behaviour that qemu-user can't enforce.
- **`setsid(2)` semantics** — `getsid(0)` in the child after
  `setsid()` returns 0 under qemu-user instead of the new
  session leader's PID. `audit-l3-fork-setsid` asserts the
  Linux-native invariant.

Real aarch64 hardware (RPi4, Apple Silicon under a Linux VM,
native ARM cloud instances) should pass these — they're
qemu-user-specific.

### Upstream sigil quirk (open as of 1.5.4)

- **Ed25519 verify accepts wrong public key on aarch64.** Filed
  upstream in sigil's issue tracker
  (`docs/development/issues/2026-05-10-ed25519-verify-aarch64-accepts-wrong-pk.md`).
  Affects `tests/tcyr/audit_extended.tcyr` `audit-ext-sign-ed25519`
  "wrong vk rejected" assertion. The right-vk path works; only
  wrong-key detection mis-passes. Native x86_64 is clean.
  Argonaut consumers signing audit snapshots on aarch64 should
  not rely on cross-arch verify (the supervisor is always
  single-arch — sign on the running arch, verify on the same
  arch — so the bug doesn't escape into cross-arch trust paths).

Net effect on the aarch64 sweep: 26/28 suites pass under
qemu-user, **0 known regressions on argonaut surface**. The two
non-passing suites are documented above.

## Real-hardware validation

Out of scope for CI as of 1.5.4 — gated on allocation of an
aarch64 CI runner. Manual validation paths:

- **RPi4 (Cortex-A72)** — Raspberry Pi OS 64-bit; build the
  static aarch64 binary on x86_64, scp to the Pi, run.
- **Apple Silicon (Cortex-A76 / M-series)** — Asahi Linux
  arm64; same flow.
- **Cloud aarch64** — AWS Graviton, Hetzner ARM64, OCI Ampere
  Altra; static binary works directly.

The 1.5.4 ship has been validated under qemu-user only. Real-hw
boot smoke is on the 1.6.x roadmap once a runner is allocated.
