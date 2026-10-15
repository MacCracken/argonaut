# ADR-013: Service Policy Is Enforced by the Consumer, Through a Pre-Exec Hook

**Status**: Accepted (decided at 1.9.0, 2026-08-24; recorded 2026-10-14). Supersedes the
enforcement parts of ADR-010 and ADR-011.
**Date**: 2026-10-14

## Context

`ServiceDefinition` has carried `seccomp`, `landlock` and `capabilities` fields since the
struct was written. ADR-011 planned to enforce them through agnosys, and ADR-010 planned
to apply resource limits by running `prlimit(1)` after spawn. Neither path was ever
built:
- agnosys left the dependency graph at 1.8.4;
- nothing in the spawn path runs the `prlimit` commands.

The 2026-08-24 audit (DOC-1) corrected the docs: argonaut enforces none of these.

kybernet, the production PID 1, already had about 686 lines of seccomp, Landlock and
privilege-drop code. Its own 2026-08-24 audit (HIGH-1) found that code reachable from no
production path. The policy was not missing; the seam was. A seccomp filter, a Landlock
ruleset or a capability drop can only be installed in the child between `fork` and
`execve`, and argonaut owns that window.

## Decision

argonaut supplies the window; the consumer supplies the policy.

- **The hook.** `argonaut_set_pre_exec_hook(fp)` registers a callback (1.9.0, in
  `src/process_mgmt.cyr`). `fork_exec_service` calls it in the child, after
  `reset_child_signal_mask()` and immediately before `sys_execve`, and passes the
  `ServiceDefinition`.
- **Fail closed.** A non-zero return aborts the child with exit **126**, distinct from
  127 ("exec failed"). Running a service unconfined when its policy failed to apply is
  worse than not running it.
- **Opaque fields.** argonaut does not interpret `seccomp`, `landlock`, `capabilities`
  or `cgroup_limits`. They are opaque pointers with accessors, and 0 means "no policy".
  The consumer defines their layout; kybernet stores agnostik types in them.
- **What argonaut does with them.** It renders descriptions (`seccomp_description`,
  `landlock_description`) and keeps the kernel capability numbers (`capability_str` /
  `capability_from_str`). It applies nothing.

kybernet's `kyb_pre_exec` (`src/lib/service_sandbox.cyr`) is the implementation. It
applies cgroup placement, `no_new_privs`, capabilities, the uid/gid drop, Landlock and
seccomp, in that order, opt-in per service.

## Consequences

- **Positive**: one implementation of the security model, in the component that has an
  opinion about it. argonaut stays a service manager.
- **Positive**: the window is placed correctly once, for every consumer. It sits after
  the signal reset, so a seccomp filter need not allow `rt_sigprocmask`, and it is last
  before `execve`, so whatever it installs holds for the exec'd image.
- **Negative**: **with no hook registered, a service runs with PID 1's full root
  privileges.** argonaut's own binary starts no services, so this bites only a consumer
  that skips the hook. Never describe argonaut itself as sandboxing services.
- **Negative**: other argonaut exec paths do not run the hook. A command health check
  (`check_command` → `run_safe_cmd_timeout`) for a confined service runs unconfined; the
  roadmap pins this at 1.18.0.
- **Negative**: the policy fields have no layout argonaut can check. A consumer that
  stores the wrong type finds out in its own hook.

## Alternatives Considered

- **Enforce in argonaut** (ADR-011's plan, or a native port of it). This duplicates
  kybernet's working code and puts one consumer's policy model (agnostik's types) inside
  the service manager. Revisit only if a second consumer needs enforcement and cannot
  write a hook. That change would supersede this ADR.
- **Post-spawn application** (`prlimit(1)`, ADR-010). There is a race between `exec`
  and the external command. It also cannot install seccomp or Landlock, which must be
  done by the process itself.
- **Generate a `setpriv` wrapper command** (`capability_setpriv_cmd`). It adds a
  dependency on an external binary at the security boundary, and nothing runs it.
