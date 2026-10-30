# ADR-014: argonaut Applies Resource Limits and Env Files at Spawn, and Checks Names at Registration

**Status**: Accepted. Supersedes ADR-010; narrows the resource-limit part of ADR-013.
**Date**: 2026-10-29
**Decision Makers**: the maintainer (1.17.0: wire the inert helpers, do not remove them)

## Context

Three parts of `ServiceDefinition` had helpers and no caller (the 2026-10-14 doc sweep;
roadmap 1.17.0, "inert helpers"):
- **Resource limits.** The `resource_limits` field, `svc_def_rlimits`,
  `to_prlimit_commands` and `resource_limits_secure` existed. ADR-010 planned to apply
  limits after spawn with `prlimit(1)`, and nothing ever ran those commands. ADR-013 then
  recorded that argonaut applies no policy.
- **Env files.** The `env_files` field, `load_env_file` and `load_env_files` existed. The
  spawn path never read the field. kybernet loads its own `env_files` config key with
  argonaut's `load_env_file`, at config load.
- **Service names.** `validate_service_name` existed and nothing called it. A name reaches
  the unit generator, the audit chain, and kybernet's cgroup path.

Each one suggested a feature argonaut did not have. The roadmap left it open whether to
remove each or wire it; the decision was to wire them.

## Decision

**Resource limits are applied in the spawn child.** `fork_exec_service` calls
`_apply_rlimits(svc_def_rlimits(sd))`:
- after argv and envp are built, so an address-space limit cannot starve those
  allocations;
- after the signal-mask reset;
- before the pre-exec hook, which may drop `CAP_SYS_RESOURCE`.

Each field that is set becomes both the soft and the hard limit, through `prlimit64(0, …)`
(`ag_sys_prlimit64`, x86_64 302 / aarch64 261). That is what `prlimit --nofile=N` sets.
The fields:
- NOFILE, AS and NPROC are set when > 0;
- CORE is set when >= 0, so 0 means no core dumps.

A limit the kernel refuses fails the child closed with **126**, the hook's code.
`svc_rlimits_new` and `svc_def_set_rlimits` make the field settable.

**Env files are loaded into the service's environment.**
- **The merge:** `build_service_envp` builds one map: the `environment` entries first,
  then each env file in order, a later entry overriding an earlier one. That is systemd's
  Environment= / EnvironmentFile= order and kybernet's. A file entry also replaces a
  default of the same name (PATH).
- **The parent preflight:** `fork_exec_service` checks every path before it forks. Each
  must be absolute, a regular file and at most 8 KiB, and the check allocates nothing.
  The caller sees -1 and audits the failure.
- **The child read:** the child reads the files for real. A file that cannot be loaded,
  or that names a variable outside `[A-Za-z_][A-Za-z0-9_]*`, fails it closed with 126.
- **Opening:** every open is `O_NONBLOCK` with an `S_ISREG` check (the B-1 class), and a
  symlink is followed, as systemd's EnvironmentFile= follows one.
- **The setter:** `svc_def_set_env_files` makes the field settable.

**Names are checked where services enter.** `argonaut_init_new` registers no service
whose name `validate_service_name` refuses:
- the allowed set is `[A-Za-z0-9._-]`;
- an empty name, a bare `.` and any `..` are refused.

Each refusal is an `EVT_REGISTRATION_REFUSED` ("RegistrationRefused") audit record, at
error severity.

**ADR-013 is unchanged for security policy.** `seccomp`, `landlock`, `capabilities` and
`cgroup_limits` stay opaque, and the consumer's hook applies them.

## Consequences

- **Positive**: a field that can be set now takes effect. There are no more silent
  no-ops in the definition a consumer fills.
- **Positive**: the limits and the environment are in place before the hook runs, so a
  consumer's hook sees the service's final process state.
- **Positive**: a service cannot hang PID 1 by replacing its env file with a FIFO. Before
  this, kybernet's own `load_env_file` call was exposed to that too.
- **Negative**: a service that has a limit or an env file which cannot be applied no
  longer starts. That is the point, but a broken config now shows up as a refused start or
  a 126 exit.
- **Negative**: a name that kybernet accepts (it refuses only `.` and `..`) but argonaut
  refuses, such as `a..b`, is not registered. Each such name leaves an audit record.
- **Negative**: `struct ResourceLimits` still collides with agnostik's struct of the same
  name. cyrius keeps the first definition and warns, so a consumer must fill the field
  with argonaut's constructors, never agnostik's.
- **Neutral**: `to_prlimit_commands` stays. It renders the same limits as `prlimit(1)`
  command lines for a pid that is already running; the spawn path does not use it.

## Alternatives Considered

- **Remove the helpers.** Rejected: a field a consumer can set should do what it says.
- **Post-spawn `prlimit(1)`** (ADR-010). Rejected for the same reason ADR-013 gave: it
  races the exec, and it depends on an external binary.
- **Leave limits to the consumer's hook.** A hook can call `setrlimit` itself. But the
  field is argonaut's, and a hook that drops `CAP_SYS_RESOURCE` first could not raise a
  hard limit afterwards. Applying limits before the hook gets the order right for every
  consumer.
- **Read env files in the parent.** Rejected: `alloc` never frees, and PID 1 restarts
  services indefinitely. The parent only checks the files, without allocating; the child,
  whose memory dies with the exec, reads them.
- **Validate names in `svc_def_new`.** Rejected: it returns a definition and has no way to
  refuse. Registration is where a name enters, and where a refusal can be audited.
