# 001 — Cross-arch (aarch64) build surface

Status: Active (since 1.5.4, 2026-05-10)

## Summary

Argonaut cross-builds to aarch64 with cyrius's `cycc_aarch64`.

**Portability is argonaut's job, not the compiler's.** The aarch64 backend does carry an
x86-compat translation ladder for some syscall numbers, but relying on it is a trap: at
1.13.1, nine literal x86 numbers were found landing on the wrong aarch64 syscall. Every
syscall goes through a stdlib `sys_*` wrapper, or through an arch-dispatched `ag_sys_*`
entry in `src/syscall_compat.cyr`. That file's header records the ESYSXLAT rules, and
each of its numbers was verified under `qemu-aarch64 -strace`. A literal `syscall(N, …)`
is a defect.

Every release ships it as `argonaut-<VER>-aarch64-linux` beside the x86_64 build. It is
required: a toolchain without `cycc_aarch64`, a failed cross-build, or a mismatch with the
native build (below) fails CI and the release.

## Producer

```
CYRIUS_DCE=1 cyrius build --aarch64 src/main.cyr build/argonaut-aarch64
```

The aarch64 binary is about 3.5× the x86_64 one. Most of the gap is DCE: on aarch64 it
fills dead code with NOPs instead of compacting it, because there is no compaction repair
model yet. Current sizes are in `docs/development/state.md` (Binary).

## CI on arm64 hardware

The `aarch64-native` job in `.github/workflows/ci.yml` runs on a GitHub-hosted
`ubuntu-26.04-arm` runner:
- it installs cyrius's aarch64-linux tarball, a native `cycc` and `cyrius`;
- it builds `src/main.cyr` there, and requires the result to be **byte-identical** to the
  x86_64 job's cross-build;
- it smoke-tests that binary (`all systems nominal`) and runs every `.tcyr` suite natively.

**The byte-identity is what makes the release artifact a tested one.** The release ships
the cross-built binary, and rebuilds it from the tag only to `cmp` it against the bytes CI
tested. A toolchain whose native and cross compilers disagree fails CI instead of shipping
a binary no suite ran.

## Known-failure surface

**None since 1.13.1.** `scripts/aarch64-sweep.sh` runs every `.tcyr` suite under
qemu-user with no known-failure allowance, so any failure is a regression. CI runs the same
suites natively (above), with the same rule. The current
count, identical to x86_64, is in `docs/development/state.md` (Suites).

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

CI covers a GitHub-hosted arm64 runner (above). Other hardware is checked by hand:

- **RPi4 (Cortex-A72):** `scripts/aarch64-pi-smoke.sh` builds the static binary on
  x86_64, copies it to the Pi and runs it. It has been validated by hand (argonaut
  initialises in ~536 µs).
- **Apple Silicon (Asahi Linux arm64)** and **cloud aarch64** (Graviton, Ampere): the
  same static binary runs directly.
