# Feature Gap Analysis

This is a checklist of what production init systems (s6, dinit, systemd, runit, OpenRC)
provide, and where each item stands in argonaut today. "Consumer" means the work is done
in kybernet, through argonaut's pre-exec hook or kybernet's own config loading
([ADR-013](../adrs/013-policy-enforcement-in-the-consumer.md)). Work that is still open
is pinned in [`roadmap.md`](../development/roadmap.md).

## P0 — Required Before Production

| Feature | Status |
|---------|--------|
| Zombie reaping | **Implemented.** `init_reap_services` plus the orphan sweep `proc_table_reap_orphans`, with PR_SET_CHILD_SUBREAPER enrolment (1.5.0). argonaut's own PID-1 loop reaps on SIGCHLD from a signalfd (1.6.0+) |
| Signals and shutdown | **Implemented.** argonaut's PID 1 treats SIGTERM as poweroff and SIGINT (Ctrl-Alt-Del) as reboot, and syncs first. `init_execute_shutdown` / `init_stop_all` stop services in reverse dependency waves within one budget (1.16.2) |
| Parallel startup | **Implemented.** `init_boot_execution_plan_waves`: one Kahn's pass, deterministic order inside each wave (1.15.6 / 1.16.1). The consumer runs the waves |
| Cgroup per service | **Consumer.** argonaut stores an opaque `cgroup_limits` (1.13.0); kybernet joins the cgroup in its pre-exec hook |
| Resource limits (rlimits) | **Implemented (1.17.0, ADR-014).** `fork_exec_service` applies `svc_def_rlimits` in the child before the pre-exec hook: NOFILE, AS, NPROC and CORE, each as both soft and hard limit, and a refused limit fails the start closed (126). Set with `svc_rlimits_new` / `svc_def_set_rlimits`. kybernet also limits services through `cgroup_limits` |
| Privilege drop (uid / gid) | **Consumer.** argonaut has no uid / gid field; kybernet drops privileges in its pre-exec hook |
| Forking services | **Implemented.** `SVC_FORKING`: the PID is read from the PID file strictly (no symlinks or FIFOs; numeric only; 1.16.1), and the start is bounded by a 90 s default timeout |

## P1 — Should Have

| Feature | Status |
|---------|--------|
| sd_notify readiness | **Implemented.** `SVC_NOTIFY` (1.13.6) stays STARTING until an authenticated `READY=1` |
| sd_notify `WATCHDOG=1` | **Implemented.** An authenticated keepalive refreshes the deadline from `watchdog_ms`, and `init_enforce_watchdog` kills a service that misses it |
| sd_notify sender authentication | **Implemented.** `notify_bind` sets `SO_PASSCRED`. `notify_try_recv_authenticated` matches the sender against the service's pids, drops truncated datagrams and closes passed descriptors. The unauthenticated `notify_try_recv` / `notify_drain` were removed at 1.17.0 |
| sd_notify `RELOADING=1`, `STOPPING=1`, `MAINPID=`, `STATUS=` | **Parsed, not acted on.** Only `READY=1` and `WATCHDOG=1` change state |
| Socket activation | **Not implemented.** `socket_env_listen_fds` builds `LISTEN_FDS` / `LISTEN_PID` strings, but nothing creates or passes sockets, and the `socket_activation` field is never read |
| Log rotation | **Not implemented.** `LogConfig` is a struct with a default and no reader. Service stdout and stderr go to `/dev/null` |
| Environment files | **Implemented (1.17.0, ADR-014).** `fork_exec_service` loads `svc_def_env_files` into the service's environment: the `environment` map first, then each file in order, a later entry overriding an earlier one, and a file entry replacing a default of the same name. A missing, oversized, non-absolute or non-regular file refuses the start; a file the child cannot load, or one naming an invalid variable, fails it closed (126). There is no implicit `env.d`. kybernet loads its own `env_files` key with the same `load_env_file` |
| tmpfiles.d equivalent | **Implemented.** `tmpfiles_apply` creates directories, symlinks and device nodes natively, without following a symlink in any path component (1.16.2). `generate_tmpfile_commands` still renders the command form |
| Seccomp / Landlock / capabilities | **Consumer.** argonaut models the policy (opaque fields, description renderers) and enforces none of it. `fork_exec_service`, and a service's command checks (1.18.0), pass each definition to the pre-exec hook, which fails closed (exit 126). kybernet's hook applies `no_new_privs`, capabilities, Landlock and seccomp per service. **With no hook, a service runs with PID 1's full root privileges** |
| Health-check commands under policy | **Implemented (1.18.0, ADR-015).** A service's `HC_COMMAND` health or ready check runs with the service's environment, env files and resource limits, under its pre-exec hook (`argonaut_exec_kind() == EXEC_HEALTH_CHECK`). A refusal fails the check closed |

## P2 — Nice to Have

| Feature | Note |
|---------|------|
| D-Bus interface | Only if the AGNOS desktop needs it (roadmap non-goal) |
| Timer-based services | samay's domain (roadmap non-goal) |
| On-demand lazy start | Depends on socket activation |
| Transient units | One-off commands in cgroups (like `systemd-run`) |
| SBOM generation | Supply-chain compliance |
| Reproducible builds / provenance | SLSA; releases publish SHA256SUMS only today |

## Edge boot and sd_notify details

### dm-verity
- `veritysetup open` verifies on read; there is no redundant `verify` pass.
- `--restart-on-corruption` is used in edge mode, and a verity failure is fatal there.
- FEC (`--fec-device`) is not implemented; it is gated in the roadmap.

### LUKS2 + TPM2
- The unlock is `cryptsetup open --type luks2 --token-only --token-id 0 --tries 1 --`:
  the key comes only from the LUKS2 token, with one attempt and no passphrase fallback.
- `pcr_bindings` is **not** a sealing policy. It is a read-out set that kybernet compares
  for drift, and argonaut has no TPM code. Which boot states the TPM releases the key to
  is set at enrolment: see
  [architecture note 005](005-edge-boot-tpm-key-policy.md).

### sd_notify
- The abstract socket namespace is not supported; it is gated in the roadmap.

## Process shape

kybernet links argonaut's `src/` **in-process**. It is one PID-1 process with its own
event loop, config loading and policy hook. There is no separate "argonaut-manager"
process. A two-process split (a minimal PID 1 that execs a manager) was the original
recommendation from this analysis, and it was not adopted.

argonaut's own binary is a minimal PID 1. It mounts, takes signals from a signalfd,
reaps, syncs and powers off. It does not run the service lifecycle.
