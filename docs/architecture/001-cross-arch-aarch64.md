# 001 — Cross-arch (aarch64) build surface

Status: Active (since 1.5.4, 2026-05-10)

## Summary

Argonaut cross-builds to aarch64 via cyrius's
`cycc_aarch64` translator (renamed from `cc5_aarch64` in
Cyrius 6.0, when the host compiler renamed `cc5` → `cycc`). No
argonaut source changes are required — the toolchain converts
x86_64 syscall numbers + ABI to aarch64 at codegen time.
Binary ships in CI / release artifacts as
`argonaut-<VER>-aarch64-linux` alongside the x86_64 build,
**best-effort** (skipped without failing the run when the
toolchain release didn't bundle `cycc_aarch64`).

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

- Skips with a warning when `cycc_aarch64` isn't in the toolchain
  bin dir.
- Skips with a warning when neither `qemu-aarch64-static` nor
  `qemu-aarch64` is on PATH.

## Known-failure surface

**None as of 1.13.1.** The aarch64 qemu-user sweep is 30/30 suites /
872 assertions — identical to native x86_64.

### Superseded: the "qemu-user emulation limits" claim (1.5.4 - 1.13.0)

This section previously excused two suites, `audit_findings.tcyr` and
`audit_extended.tcyr`, attributing them to qemu-user's process model and
an upstream sigil ed25519 quirk. **The qemu-user half of that diagnosis
was wrong** — those were real, argonaut-side aarch64 defects, fixed in
1.13.1:

- `audit-m3-reaper-orphans` did not fail because "reparenting under
  PR_SET_CHILD_SUBREAPER doesn't compose with the emulator". It failed
  because `syscall(157, 36, 1, ...)` is `prctl` only on x86_64; on
  aarch64 157 is **`setsid`**, so the subreaper was never enrolled. The
  test's own 50 ms/200 ms settle sleeps were `syscall(35, ...)`, which is
  `nanosleep` on x86_64 and **`unlinkat`** on aarch64 — they returned
  -EFAULT instantly and never slept.
- `audit-l3-fork-setsid` did not fail because "getsid(0) returns 0 under
  qemu-user". It failed because `syscall(112)` is `setsid` only on
  x86_64; on aarch64 it is **`clock_settime`**, and `syscall(124)` is
  `getsid` on x86_64 but **`sched_yield`** on aarch64. The child never
  became a session leader and never read back a session id.
- `audit_extended` was not failing on ed25519 at all by the time it was
  re-measured. Its aarch64 failure was `persist: open succeeds`: the
  suite cleans up scratch files with `unlink`, which is `syscall(87)` on
  x86_64 but **`timerfd_gettime`** on aarch64, so stale state from a
  previous run was never removed and the reopen failed. The separate
  ed25519 "wrong vk rejected" assertion passes on aarch64 both before
  and after this change — that upstream sigil issue was resolved
  independently (sigil 3.12.9) and is not attributable to this fix.
- The pid-file group/other-writable assertions failed because
  `read_pid_file_safe` read `st_mode`/`st_uid` at the **x86_64**
  `struct stat` offsets (+24/+28). On the aarch64 asm-generic layout
  those hold `st_uid`/`st_gid`, so for a root-owned file both read 0,
  the `mode & 0o022` test passed, and the CVE-2025-4598-class check
  **failed open**.

All of these were verified against the real kernel entry with
`qemu-aarch64 -strace`, not inferred. The lesson worth keeping: an
emulator is a convenient thing to blame, and blaming it hid a security
check failing open on the shipping ARM binary for eight releases. Before
writing a failure off as emulation, read the syscall the kernel actually
received.

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
