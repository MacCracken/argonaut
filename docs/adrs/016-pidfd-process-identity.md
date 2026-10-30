# ADR-016: Tracked Processes Are Identified by a pidfd, with Probe-Then-Signal as the Fallback

**Status**: Accepted.
**Date**: 2026-10-30
**Decision Makers**: the maintainer (1.18.0: fall back to B-7 rather than refusing to start
services on a kernel without pidfd)

## Context

The proc table maps a service name to a pid **number**, and the kernel reuses a number once
its process has been reaped. The 2026-10-14 audit's B-7 made every stop path probe first
(`waitpid(pid, WNOHANG)`) and signal only argonaut's own running child. That is exact while
argonaut is the only reaper of its children, and argonaut is single-threaded. It is not
exact when something else reaps a tracked process:
- a consumer's own `waitpid(-1)`, or a caller of the public `proc_table_*` API;
- a forking service's daemon, which is not argonaut's child unless argonaut is PID 1 or a
  subreaper.

Then the entry keeps a number that may name another process, even another child of
argonaut's, which the probe cannot tell apart. The watchdog had no probe at all: it sent
`SIGKILL` to the number.

Linux 5.3 added `pidfd_open`, a file descriptor that names one process for its lifetime.
`pidfd_send_signal` (5.1) signals through it, and fails with `ESRCH` once the process has
been reaped.

## Decision

**Every proc-table entry holds a pidfd**, opened by `proc_table_insert` and closed when the
entry leaves: `proc_table_remove`, `proc_table_reap`, `proc_table_stop_all`, a re-insert,
`proc_table_init`. `pidfd_open` sets `O_CLOEXEC`, so no service inherits one.

**The pidfd is asked first.** While its process exists, alive or a zombie, the number
cannot be reused. So:
- `proc_pidfd_gone(fd)` answers "reaped elsewhere"; then the number is never probed,
  waited on or signalled. A stop returns -1 at once. `proc_table_reap` ends the entry with
  its status lost (-1). The ordered shutdown marks the member done.
- Otherwise the number still names the process, so the B-7 probe and the process-group
  kill (`kill(-pid)`, which no pidfd can express) stay exact.
- Every signal goes through `proc_send_signal`, which uses the pidfd.
- The watchdog needs no probe with a pidfd: its identity is pinned, so it kills a forking
  service's daemon that is not argonaut's child too. Without one it now probes (B-7).
- An `HC_PROCESS_ALIVE` check of a service asks its pidfd.

**Fallback: probe-then-signal (B-7).** With no pidfd (Linux < 5.3, which is warned once;
`pidfd_open` failing; or `argonaut_set_pidfd_enabled(0)`), argonaut signals by number after
the probe, as before 1.18.0.

## Consequences

- **Positive**: a tracked process's signals reach it or nothing, whoever reaped it.
- **Positive**: an entry whose process was reaped elsewhere ends, instead of staying
  RUNNING with a dead number forever.
- **Negative**: one fd per tracked process in PID 1.
- **Negative**: the fallback keeps the old window on old kernels. For argonaut's own
  children it is still exact while argonaut is their only reaper.
- **Neutral**: the pidfd map is allocated on the first insert that opens one, not by
  `argonaut_init_new`, which would have cost every init 0.6-0.7 µs.
- **Neutral**: a forking service's pid file is read, then its pidfd opened. A number
  recycled between those two steps is still adopted; the pidfd pins whatever was adopted.

## Alternatives Considered

- **Refuse to start a service without a pidfd.** A simpler invariant: every signal goes
  through a pidfd. Rejected: PID 1 on a pre-5.3 kernel would start nothing, and B-7 is
  already exact for argonaut's own children in its single-threaded loop.
- **Probe-then-signal only (no pidfd).** Rejected: it cannot tell a recycled number that
  names another child of argonaut's from the original process.
- **Track by `/proc/<pid>/stat` start time.** Rejected: it adds a read and a parse to every
  check and still races between the read and the signal. A pidfd closes that window in the
  kernel.
