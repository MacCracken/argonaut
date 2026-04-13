# Quickstart Guide

## Building argonaut

```sh
# Requires cyrius 3.9.8+
cyrius build src/main.cyr build/argonaut

# Run
./build/argonaut

# Test
cyrius test

# Benchmark
cyrius bench
```

## Basic Usage — Boot and Manage Services

```cyrius
include "lib/alloc.cyr"
# ... (stdlib includes)
include "src/audit.cyr"
include "src/init.cyr"

fn main() {
    alloc_init();

    # Configure for server mode
    var config = argonaut_config_default();
    store64(config, BOOT_SERVER);

    # Create the init system (includes audit log)
    var init = argonaut_init_new(config);

    # Start services in dependency order
    var svc_map = load64(init + 16);
    var keys = map_keys(svc_map);
    for (var i = 0; i < vec_len(keys); i = i + 1) {
        var name = vec_get(keys, i);
        var pid = init_start_service(init, name);
        if (pid > 0) {
            println(name);
        }
    }

    # Runtime loop
    var tracker = health_tracker_new();
    while (1 == 1) {
        # Reap exited processes
        var reaped = init_reap_services(init);

        # Run health checks
        init_poll_health(init, tracker);

        # Enforce watchdog timeouts
        init_enforce_watchdog(init);

        # Sleep 1 second
        var ts[16];
        store64(&ts, 1);
        store64(&ts + 8, 0);
        syscall(35, &ts, 0);
    }

    return 0;
}
var r = main();
syscall(SYS_EXIT, r);
```

## Defining a Custom Service

```cyrius
var sd = svc_def_new(
    str_from("my-api"),
    str_from("My API server"),
    str_from("/usr/bin/my-api"));
vec_push(svc_def_args(sd), str_from("--port"));
vec_push(svc_def_args(sd), str_from("3000"));
vec_push(svc_def_deps(sd), str_from("daimon"));
vec_push(svc_def_modes(sd), BOOT_SERVER);
vec_push(svc_def_modes(sd), BOOT_DESKTOP);
store64(sd + 56, RESTART_ON_FAILURE);

# Health check: HTTP GET every 15s
var hc = health_check_new(
    HC_HTTP_GET,
    str_from("http://127.0.0.1:3000/health"),
    0, 15000, 5000, 3);
svc_def_set_health_check(sd, hc);

# Ready check: TCP connect with 10s timeout
var rc = ready_check_new(
    HC_TCP_CONNECT,
    str_from("127.0.0.1"),
    3000, 10000, 20, 500);
svc_def_set_ready_check(sd, rc);

# Restart config
var restart = restart_config_new(10, 500, 60000);
svc_def_set_restart_config(sd, restart);
```

## Edge Boot

```cyrius
var config = argonaut_config_default();
store64(config, BOOT_EDGE);

# Configure edge boot
var ec = edge_config_new(1, 1, 1, 3000);  # ro_rootfs, luks, tpm, max_boot_ms

# Execute edge boot sequence
var result = execute_edge_boot(ec,
    str_from("/dev/mmcblk0p2"),
    str_from("/dev/mmcblk0p3"),
    str_from("a1b2c3d4e5f6..."),  # root hash (64 hex chars)
    str_from("/dev/mmcblk0p4"));

# Validate edge profile
var violations = validate_edge_profile(result, 128);  # 128MB RAM limit

# Build fleet registration payload
var reg = fleet_registration_from_system(result);
```

## Shutdown

```cyrius
var plan = init_plan_shutdown(init, SHUTDOWN_POWEROFF);
var steps = load64(plan + 8);

for (var i = 0; i < vec_len(steps); i = i + 1) {
    var step = vec_get(steps, i);
    var desc = load64(step + 8);
    str_println(desc);
}
```

## Audit Trail

```cyrius
# The audit log is created automatically with argonaut_init_new
var alog = init_audit_log(init);

# All lifecycle events are recorded automatically:
# start_service → EVT_STARTING, EVT_STARTED
# stop_service → EVT_STOPPING, EVT_STOPPED_OK/FAIL
# restart_service → EVT_RESTARTING
# reap (crash) → EVT_CRASH_DETECTED
# watchdog → EVT_TIMEOUT_KILLED
# health → EVT_HEALTH_PASSED/FAILED

# Query the audit log
var q = query_new();
query_min_severity(q, SEV_ERROR);
var errors = chain_query(alog, q);

# Verify chain integrity (SHA-256)
var ok = audit_log_verify(alog);  # 1 = valid, 0 = tampered
```

## Consumers

Argonaut is a library. The PID 1 binary is [kybernet](https://github.com/MacCracken/kybernet), which calls argonaut's init functions to manage the real boot process.

Other consumers:
- **stiva** — container service lifecycle
- **sutra** — infrastructure playbook service management
- **daimon** — agent runtime (triggers runlevel switches)
- **agnoshi** — shell (queries service status via API)
