# ADR-015: A Service's Command Checks Run the Way the Service Is Spawned

**Status**: Accepted. Narrows ADR-013's "other exec paths do not run the hook".
**Date**: 2026-10-30
**Decision Makers**: the maintainer (1.18.0: run checks under the service's spawn, rather
than documenting an unconfined run)

## Context

`fork_exec_service` spawns a service through a fixed sequence in the child: its
environment and env files, its resource limits (ADR-014), then the consumer's pre-exec
hook (ADR-013), where kybernet confines it: cgroup placement, `no_new_privs`, capabilities,
the uid/gid drop, Landlock and seccomp.

An `HC_COMMAND` health or ready check took a different path: `check_command` →
`run_safe_cmd_timeout`. That path has no service definition, so:
- **no hook:** a confined service's check command ran unconfined, as PID 1's root;
- **no service environment:** PATH was its whole environment, with no env files;
- **no limits:** none of the service's resource limits applied.

A check runs on the supervisor tick for the life of PID 1, configured per service by the
same operator who configured that service's confinement. The roadmap left two answers
open: run the check under the service's hook, or make the unconfined run an explicit,
documented choice.

## Decision

**A command check of a service is spawned the way the service is.** `run_check_as(sd,
cmd, timeout)` forks a child that:
- leads its own process group, as `run_safe_cmd_timeout`'s child does, so a timeout kills
  everything the check started;
- puts stdin, stdout and stderr on `/dev/null`, as a service has;
- ends in `_exec_service_image`, the same code `fork_exec_service`'s child ends in. That
  builds the service's environment (map and env files), applies its resource limits, and
  runs the pre-exec hook with the service's definition, then execs the check.

**The hook can tell a check from the service.** `argonaut_exec_kind()` returns
`EXEC_SERVICE` or `EXEC_HEALTH_CHECK` in the child (`EXEC_NONE` everywhere else). A hook
that ignores it confines a check exactly as it confines the service, which is the
intended default.

**It fails closed.** A refused environment, limit or hook exits 126, and the check fails:
the service reads unhealthy. A check never runs unconfined in place of a refused one.

**Wiring.** `init_poll_health` calls `execute_service_health_check(sd, …)`, and
`init_start_simple` calls `execute_service_ready_check(sd, …)`. The name-only
`execute_health_check`, `execute_ready_check` and `check_command` keep their behaviour:
with no definition there is no service policy to apply, and a caller with no service
(edge boot's `run_safe_cmd_timeout`) is not a service check.

**Binary resolution stays in the parent**, against `run_safe_cmd_timeout`'s fixed search
path, so what is exec'd does not depend on a PATH the service's environment sets.

## Consequences

- **Positive**: one confinement model for everything that runs on a service's behalf.
  This is systemd's model too: a unit's `ExecStartPre=` / `ExecStartPost=` commands run
  under the unit's sandboxing, environment and limits.
- **Positive**: a check sees the environment its service sees (a database URL from an env
  file, say), which is usually what the check needs to probe the right thing.
- **Negative (Breaking)**: a check command the service's policy does not allow now fails.
  Under kybernet's hook the check runs in the service's cgroup, as the service's uid, under
  its Landlock ruleset and seccomp filter. A check binary outside the ruleset, or a check
  that relied on root, now reads unhealthy and can trip the watchdog. The fix is in the
  service's policy, or in a hook that treats `EXEC_HEALTH_CHECK` differently.
- **Negative**: check output no longer reaches PID 1's console; it goes to `/dev/null`,
  as service output does.
- **Neutral**: the parent stays allocation-free on the tick. The environment is built in
  the child, and argv is `run_safe_cmd_timeout`'s static buffer.

## Alternatives Considered

- **Unconfined, documented.** Keep `run_safe_cmd_timeout` and record the unconfined run as
  a choice. Rejected: the operator who confined the service did not choose to give its
  checks PID 1's root, and nothing in the configuration says so.
- **A separate check hook** (`argonaut_set_check_pre_exec_hook`). Rejected: an unset second
  hook would leave checks unconfined by default, the gap this closes. `argonaut_exec_kind()`
  gives a hook the same choice without that default.
- **Pass the kind as a second hook argument.** Rejected: it changes the hook's signature
  for every consumer. Reading `argonaut_exec_kind()` is opt-in.
