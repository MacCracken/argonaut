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
qemu/
├── build-initramfs.sh   — stage argonaut binary as /sbin/init
├── boot-test.sh         — qemu-system-x86_64 + marker assertion
├── initramfs/           — staging tree (gitignored)
└── initramfs.cpio.gz    — final cpio (gitignored)
```

## Producer

```
qemu/build-initramfs.sh
qemu/boot-test.sh
```

`build-initramfs.sh` rebuilds the argonaut binary first if its
manifest is newer, then stages `/sbin/init` (argonaut), bundles
busybox if present (`/usr/lib/initcpio/busybox`,
`/usr/bin/busybox`, or `/bin/busybox`), installs minimal
`/etc/hosts` (localhost only — argonaut's resolver path needs
it during boot), creates `/dev/console` + `/dev/null` +
`/dev/ttyS0` device nodes (needs sudo, skipped silently
otherwise — boot smoke still works).

`boot-test.sh` runs the cpio under `qemu-system-x86_64`, greps
stdout for three boot-completion markers, fails the run if any
is missing or if the kernel panics on "Attempted to kill init".

## Boot-completion markers

The harness greps these three strings from argonaut's stdout
via the qemu serial console (`-serial mon:stdio`):

| Marker                           | Means                                                              |
|----------------------------------|--------------------------------------------------------------------|
| `argonaut: init system ready`    | `alloc_init` succeeded; `println` first call worked                 |
| `argonaut: all systems nominal`  | `argonaut_init_new` ran end-to-end; boot info / shutdown plan printed |
| `argonaut: pid1 loop ready`      | `sys_getpid() == 1` detected; supervisor loop entered (post-1.6.0 PID-1 path) |

Missing the third marker = `getpid() != 1` (running outside
qemu) OR argonaut exited from `main()` before the loop. Missing
the first two = argonaut crashed during init.

## CPU requirement — invariant TSC

sakshi's `_sk_clock_init` (the tracing/timing stdlib argonaut
pulls transitively via libro 2.6.2) panics if CPUID 0x80000007
EDX bit 8 (invariant TSC) is absent. qemu's TCG emulation
**does not expose this bit** even with `-cpu max,+invtsc`;
only KVM with `-cpu host,+invtsc` faithfully passes it
through from the host CPU. Practical consequence:

- **Local validation:** needs `/dev/kvm` readable. The harness
  auto-detects via `[ -r /dev/kvm ]` and selects accel
  accordingly; runs that fall through to TCG print a clear
  "harness needs KVM" diagnostic and fail with the
  invariant-TSC panic.
- **CI:** GitHub Actions ubuntu-runners have `/dev/kvm`
  available on Linux runners since the 2023 nested-virt
  rollout; the workflow uses the same KVM-required path.
- **Real-hardware:** any modern x86_64 host has the bit;
  not an issue.

The TCG / qemu-user limitation here mirrors the 1.5.4
`audit-m3-reaper-orphans` / `audit-l3-fork-setsid`
known-failures documented in
[`001-cross-arch-aarch64.md`](001-cross-arch-aarch64.md):
emulator-only gaps that don't apply to real hardware.

## Kernel + initramfs sizing

`qemu/boot-test.sh` auto-picks a kernel from common paths
(`/boot/vmlinuz-linux-lts`, `/boot/vmlinuz-linux`,
`/boot/vmlinuz-$(uname -r)`, `/boot/vmlinuz`). Override
explicitly as the first positional argument.

Current sizing at 1.6.0 ship:

- argonaut binary: ~1.00 MB statically linked ELF x86_64
  (`CYRIUS_DCE=1`)
- initramfs.cpio.gz: ~376 KB (argonaut + busybox)
- qemu memory: 256 MB (`-m 256M`) — comfortable headroom;
  argonaut's working set is ~few KB

Boot wall time: ~0.3 s under KVM, ~2.5 s under TCG (kernel
load + decompress dominates; argonaut's init_new is ~50 µs).

## Future end-to-end M3 / L3 coverage

The 1.6.0 harness validates **boot reaches the supervisor
loop** — proves the PID-1 graduation path works without
landing the M3 (orphan reap under real-PID-1 reparenting) and
L3 (controlling-TTY decoupling) end-to-end variants the
2026-04-26 audit gated on this work. Those land in 1.6.1+:

- **M3 end-to-end:** spawn a busybox helper from inside
  qemu that forks a grandchild + exits; assert argonaut's
  `proc_table_reap_orphans` collects the grandchild via
  the supervisor-loop tick.
- **L3 end-to-end:** invoke `fork_exec_service` against a
  test service that writes its `getsid(0)` to a tmpfs
  marker; assert the marker reads the child's own PID
  (decoupled session).

Shape exists in `tests/tcyr/audit_findings.tcyr`
(`audit-m3-reaper-orphans`, `audit-l3-fork-setsid`) as
unit-level mock-ups; the end-to-end shape lifts them inside
qemu under real PID 1.

## Re-audit trigger

Per `docs/audit/2026-04-26-audit.md`, "argonaut graduating to
true PID 1" is one of the four re-audit triggers. 1.6.0 ships
the harness; the corresponding P(-1) re-audit pass is the
**1.6.x arc closeout** item, mirroring the 1.5.x arc shape
(scaffold lands first, closeout audit at end of the minor).
Track in `docs/development/roadmap.md`.
