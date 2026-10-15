# 002 — QEMU PID-1 boot harness

Status: Active (since 1.6.0, 2026-05-10)

## Summary

argonaut graduates to true PID 1 under qemu via the `qemu/`
harness. Lands the audit re-trigger condition flagged in
[`docs/audit/2026-04-26-audit.md`](../audit/2026-04-26-audit.md)
(re-audit on PID-1 graduation, deferred from 1.4.0). Scaffold
adapted from kybernet's `qemu/` shape per the "pull pattern,
not code" convention — argonaut's harness is standalone, no
kybernet at runtime.

## Files

```
src/pid1_harness.cyr        — the M3 / L3 self-test mode (`argonaut.harness=1` on the kernel cmdline)
qemu/
├── build-initramfs.sh      — stage argonaut as /sbin/init, plus /bin/l3-helper and busybox
├── boot-test.sh            — boot under qemu-system-x86_64; assert the boot markers
├── pid1-harness-test.sh    — boot with argonaut.harness=1; assert `m3 ok` and `l3 ok`
├── helpers/
│   ├── l3-helper.cyr       — static helper that records getsid(0) for the L3 check
│   ├── cyrius.cyml, lib/   — its own stdlib-only manifest and committed lib/
│   └── l3-helper           — the committed static binary
├── initramfs/              — staging tree (gitignored)
└── initramfs.cpio.gz       — final cpio (gitignored)
```

## Producer

```
qemu/build-initramfs.sh
qemu/boot-test.sh
qemu/pid1-harness-test.sh
```

**`build-initramfs.sh`:**
- rebuilds the argonaut binary first if its manifest is newer;
- stages `/sbin/init` (argonaut) and `/bin/l3-helper`;
- bundles busybox when present, with its dynamic loader and libc if busybox is
  dynamically linked;
- installs a minimal `/etc/hosts` (the resolver path reads it);
- creates `/dev/console`, `/dev/null` and `/dev/ttyS0`. This needs sudo, and is skipped
  without it; the boot smoke still works.

**`boot-test.sh`** runs the cpio under `qemu-system-x86_64` and greps stdout for the
boot markers below. It fails if any is missing, or if the kernel panics with "Attempted
to kill init".

**`pid1-harness-test.sh`** boots the same cpio with `argonaut.harness=1`. argonaut then
runs its self-tests inline as the real PID 1, and powers off.
- **M3:** an orphaned grandchild is reaped through PID-1 reparenting.
- **L3:** `fork_exec_service` runs `/bin/l3-helper`, which records `getsid(0)`. The
  session id must equal the child's pid, which proves `setsid`.

The script asserts `argonaut: harness m3 ok` and `argonaut: harness l3 ok`; any
`FAIL` line is fatal. The unit-level versions are the `audit-m3-reaper-orphans` and
`audit-l3-fork-setsid` groups in `tests/tcyr/audit_findings.tcyr`.

## Boot-completion markers

The harness greps these three strings from argonaut's stdout
via the qemu serial console (`-serial mon:stdio`):

| Marker                           | Means                                                              |
|----------------------------------|--------------------------------------------------------------------|
| `argonaut: init system ready`    | `alloc_init` succeeded; `println` first call worked                 |
| `argonaut: all systems nominal`  | `argonaut_init_new` ran end-to-end; boot info / shutdown plan printed |
| `argonaut: pid1 loop ready`      | `sys_getpid() == 1` detected; supervisor loop entered (post-1.6.0 PID-1 path) |

Missing the third marker means one of:
- `getpid() != 1` (running outside qemu);
- argonaut exited from `main()` before the loop;
- `signalfd` failed. It then prints `argonaut: signalfd failed` and falls back to a
  reap-only loop.

Harness mode never prints it either. Missing the first two means argonaut crashed during
init.

## CPU requirement — invariant TSC

sakshi's `_sk_clock_init` (the tracing / timing library in argonaut's stdlib
list) panics if CPUID 0x80000007
EDX bit 8 (invariant TSC) is absent. qemu's TCG emulation
**does not expose this bit** even with `-cpu max,+invtsc`;
only KVM with `-cpu host,+invtsc` faithfully passes it
through from the host CPU. Practical consequence:

- **Local validation:** needs `/dev/kvm` readable. The harness
  auto-detects via `[ -r /dev/kvm ]` and selects accel
  accordingly; runs that fall through to TCG print a clear
  "harness needs KVM" diagnostic and fail with the
  invariant-TSC panic.
- **CI:** no workflow runs the harness yet. It is a local gate run at every release,
  and running it in CI (where KVM is available) is roadmap 1.17.2.
- **Real-hardware:** any modern x86_64 host has the bit;
  not an issue.

This TCG gap is a real emulator limitation. The aarch64 failures once excused as "qemu-user
limits" were not: they were real defects, fixed at 1.13.1 (see
[`001-cross-arch-aarch64.md`](001-cross-arch-aarch64.md)).

## Kernel + initramfs sizing

`qemu/boot-test.sh` auto-picks a kernel from common paths
(`/boot/vmlinuz-linux-lts`, `/boot/vmlinuz-linux`,
`/boot/vmlinuz-$(uname -r)`, `/boot/vmlinuz`). Override
explicitly as the first positional argument.

Memory: the scripts boot at `-m 256M`, which they have passed since 1.15.5 (cyrius
6.6.8's `MAP_NORESERVE` heap). The boot test has also booted at `-m 128M`. Binary sizes
and the latest pass are in `docs/development/state.md`. The initramfs size depends on the
host's busybox.

Boot wall time is ~0.3 s under KVM, dominated by kernel load and decompression. Under
TCG the boot panics on the invariant-TSC check (above), so there is no TCG timing.

## Re-audit trigger

The 2026-04-26 audit's "argonaut graduates to true PID 1" trigger closed at 1.6.3, with
the closeout audit (`docs/audit/2026-05-11-audit.md`).

The live trigger is different. The shipped PID-1 loop still does not call the service
lifecycle. The day it calls `init_start_service`, `init_poll_health` or
`init_notify_bind`, re-rate the severities bounded by that fact (roadmap, Standing
rules).
