# Quickstart Guide

## Building argonaut

```sh
cyrius deps                                   # resolve lib/ from cyrius.cyml
cyrius build src/main.cyr build/argonaut      # build
for t in tests/tcyr/*.tcyr; do cyrius test "$t"; done   # every suite
cyrius bench tests/bcyr/argonaut.bcyr         # benchmarks
```

The toolchain version is pinned in `cyrius.cyml` (`[package] cyrius`).

## Using argonaut as a library

A consumer declares argonaut as a dependency. Its modules are listed in `src/main.cyr`'s
include order, because cyrius resolves references in one pass. This is kybernet's
shape:

```toml
[deps.argonaut]
git = "https://github.com/MacCracken/argonaut.git"
tag = "<release>"
modules = [
    "src/syscall_compat.cyr",
    "src/types.cyr",
    "src/boot.cyr",
    "src/services.cyr",
    "src/process_mgmt.cyr",
    "src/resolver.cyr",
    "src/health.cyr",
    "src/notify.cyr",
    "src/audit.cyr",
    "src/audit_ext.cyr",
    "src/init.cyr",
    "src/security.cyr",
]
```

Add `src/edge_boot.cyr`, `src/systemd.cyr` and `src/tmpfiles.cyr` if you use them.
- **Stdlib modules** come from the consumer's own `[deps] stdlib`, never from `include`
  lines in source.
- **sigil:** from 1.16.2, `src/security.cyr` needs sigil's `dist/sigil-argon2.cyr`. Declare
  `[deps.sigil]` at libro's sigil tag, as argonaut's own `cyrius.cyml` does.

## Boot and manage services

```cyrius
fn main() {
    alloc_init();

    # Server mode. Keep the audit chain queryable: the default chain streams,
    # so it hashes every event but retains none (see "Audit trail").
    var config = argonaut_config_default();
    config_set_boot_mode(config, BOOT_SERVER);
    config_set_audit_retain(config, 1);

    var init = argonaut_init_new(config);

    # Start services in dependency order. Each entry is a 16-byte pair:
    # +0 the name (Str), +8 the ServiceDefinition.
    # init_boot_execution_plan_waves groups them into parallel waves instead.
    var plan = init_boot_execution_plan(init);
    for (var i = 0; i < vec_len(plan); i = i + 1) {
        var name = load64(vec_get(plan, i));
        var pid = init_start_service(init, name);
        if (pid > 0) { str_println(name); }
    }

    # Supervisor loop
    var tracker = health_tracker_new();
    while (1 == 1) {
        init_reap_services(init);          # reap exited services
        init_poll_health(init, tracker);   # health checks; the returned vec is
                                           # valid until the next call
        init_enforce_watchdog(init);       # kill services that missed WATCHDOG=1
        sleep_ms(1000);                    # never a raw syscall number
    }
    return 0;
}
var r = main();
syscall(SYS_EXIT, r);
```

## Defining a custom service

```cyrius
var sd = svc_def_new(
    str_from("my-api"),
    str_from("My API server"),
    str_from("/usr/bin/my-api"));          # an absolute path
vec_push(svc_def_args(sd), str_from("--port"));
vec_push(svc_def_args(sd), str_from("3000"));
vec_push(svc_def_deps(sd), str_from("daimon"));
vec_push(svc_def_modes(sd), BOOT_SERVER);
vec_push(svc_def_modes(sd), BOOT_DESKTOP);
svc_def_set_restart_policy(sd, RESTART_ON_FAILURE);

# Health check: HTTP GET every 15 s, 5 s timeout, 3 retries
#   (type, target, port, interval_ms, timeout_ms, retries)
var hc = health_check_new(
    HC_HTTP_GET,
    str_from("http://127.0.0.1:3000/health"),
    0, 15000, 5000, 3);
svc_def_set_health_check(sd, hc);

# Ready check: TCP connect, 10 s timeout, 20 retries 500 ms apart
#   (type, target, port, timeout_ms, retries, retry_delay_ms)
var rc = ready_check_new(
    HC_TCP_CONNECT,
    str_from("127.0.0.1"),
    3000, 10000, 20, 500);
svc_def_set_ready_check(sd, rc);

# Restart config: start from the default and override
# (+0 max_restarts, +8 base_delay_ms, +16 max_delay_ms)
var restart = restart_config_default();
store64(restart, 10);
store64(restart + 8, 500);
store64(restart + 16, 60000);
svc_def_set_restart_config(sd, restart);
```

## Edge boot

```cyrius
# Defaults: read-only rootfs, LUKS, TPM attestation, 3000 ms budget, PCRs "7+14"
var ec = edge_config_default();

var result = execute_edge_boot(ec,
    str_from("/dev/mmcblk0p2"),    # rootfs data device
    str_from("/dev/mmcblk0p3"),    # verity hash device
    str_from("a1b2c3d4e5f6..."),   # root hash: 64 hex digits
    str_from("/dev/mmcblk0p4"));   # LUKS device

var violations = validate_edge_profile(result, 128);   # 128 MB RAM limit
var reg = fleet_registration_from_system(result);
```

The LUKS unlock uses the volume's TPM token. Which boot states the TPM will release that
key to is set at enrolment, not by argonaut; see
[architecture note 005](../architecture/005-edge-boot-tpm-key-policy.md).

## Shutdown

`init_execute_shutdown` stops every managed service in reverse dependency waves within
one budget. Each wave gets SIGTERM, then a process-group SIGKILL for stragglers. Every
service ends STOPPED and audited.

```cyrius
var stopped = init_execute_shutdown(init, 10000);   # 10 s budget
```

`init_plan_shutdown` describes the whole shutdown sequence (wall message, stop, sync,
unmount and the rest) without running it:

```cyrius
var plan = init_plan_shutdown(init, SHUTDOWN_POWEROFF);
var steps = load64(plan + 8);                 # ShutdownPlan.steps
for (var i = 0; i < vec_len(steps); i = i + 1) {
    str_println(load64(vec_get(steps, i)));   # ShutdownStep.description (+0)
}
```

## Audit trail

`argonaut_init_new` creates the audit chain.

**By default it streams.** Every event is hashed into the chain, but no entry is kept,
because PID 1 never exits and a retained chain grows forever. On a streaming chain,
`chain_query` returns nothing and `audit_log_verify` has nothing to check. To query or
verify in process, set `config_set_audit_retain(config, 1)` (as above), or use the
persistent store (`audit_persist`).

```cyrius
var alog = init_audit_log(init);

# Lifecycle events are recorded automatically:
#   start    -> EVT_STARTING, EVT_STARTED
#   stop     -> EVT_STOPPING, EVT_STOPPED_OK / EVT_STOPPED_FAIL
#   restart  -> EVT_RESTARTING; refused at the limit -> EVT_RESTART_LIMIT
#   crash    -> EVT_CRASH_DETECTED
#   watchdog / shutdown kill -> EVT_TIMEOUT_KILLED
#   health   -> EVT_HEALTH_PASSED / EVT_HEALTH_FAILED: the first result, each
#               change, and an hourly heartbeat while passing

var q = query_new();
query_min_severity(q, SEV_ERROR);
var errors = chain_query(alog, q);

var ok = audit_log_verify(alog);   # 1 = valid, 0 = tampered
```

## Confining services

argonaut does not apply seccomp, Landlock or capability policy itself. Register a
pre-exec hook. It runs in the child after fork, immediately before `execve`, and receives
the `ServiceDefinition`:

```cyrius
fn my_pre_exec(svc_def) {
    # apply the service's policy; return 0 to exec, non-zero to refuse
    return 0;
}
argonaut_set_pre_exec_hook(&my_pre_exec);
```

A non-zero return aborts the child with exit 126, and the service never runs unconfined.
kybernet's `src/lib/service_sandbox.cyr` is a complete hook. It applies cgroup placement,
`no_new_privs`, capabilities, Landlock and seccomp, in that order.
