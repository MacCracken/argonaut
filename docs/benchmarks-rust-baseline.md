# Rust Benchmark Baseline (pre-Cyrius port)

> **Historical reference only.** The Rust implementation (`rust-old/`) was removed at v0.96.1.
> This document exists solely as a baseline comparison for the Cyrius port performance analysis.
> Do not use these numbers for current performance claims — see `scripts/bench-history.sh` for
> current Cyrius benchmarks.

Captured from Criterion cache in `rust-old/target/criterion/`.
Rust edition 2024, release mode, x86_64 Linux.
These are the last benchmark numbers before the codebase was ported to Cyrius at v0.95.0.

## Side-by-Side: Rust vs Cyrius

| Benchmark | Rust (ns) | Cyrius (ns) | Ratio | Notes |
|-----------|-----------|-------------|-------|-------|
| **Boot sequence construction** | | | | |
| build_boot_seq_desktop | 213 | 4,000 | 18.8x | |
| build_boot_seq_server | 220 | 4,000 | 18.2x | |
| build_boot_seq_minimal | 142 | 4,000 | 28.2x | |
| build_boot_seq_edge | 129 | 4,000 | 31.0x | |
| **Init construction** | | | | |
| init_new_desktop | 4,935 | 23,000 | 4.7x | |
| init_new_minimal | 623 | 8,000 | 12.8x | |
| init_new_edge | 1,026 | 9,000 | 8.8x | |
| **Dependency resolution** | | | | |
| resolve_order_chain_10 | — | 9,000 | — | No Rust equivalent (chain_20 instead) |
| resolve_order_chain_20 | 5,203 | — | — | No Cyrius equivalent (chain_10/50/100) |
| resolve_order_chain_50 | — | 88,000 | — | Cyrius only |
| resolve_order_chain_100 | 30,072 | 199,000 | 6.6x | |
| resolve_order_desktop | 1,824 | 10,000 | 5.5x | |
| resolve_waves_desktop | 1,996 | 13,000 | 6.5x | |
| resolve_waves_chain_20 | 6,749 | 55,000 | 8.1x | |
| resolve_waves_wide_20 | 2,688 | 14,000 | 5.2x | |
| **Shutdown planning** | | | | |
| plan_shutdown_desktop | 2,132 | — | — | Cyrius uses reboot/poweroff |
| plan_shutdown_reboot | — | 13,000 | — | Cyrius only |
| plan_shutdown_poweroff | — | 13,000 | — | Cyrius only |
| plan_shutdown_edge | 419 | — | — | No Cyrius equivalent |
| **Health tracking** | | | | |
| health_tracker_100_checks | 5,137 | — | — | Different granularity |
| health_tracker_record | — | 510 | — | Single record (Cyrius) |
| health_history_record | — | 479 | — | Cyrius only |
| **State transitions** | | | | |
| state_transitions | 716 | 415 | 0.58x | **Cyrius faster** |
| **Backoff computation** | | | | |
| backoff_delay_compute | — | 503 | — | Cyrius only (6 delays) |
| **Systemd unit generation** | | | | |
| generate_unit (single) | — | 6,000 | — | Single service |
| generate_unit (all desktop) | 5,162 | — | — | All desktop services |
| **Edge boot** | | | | |
| configure_readonly_rootfs | 461 | 2,000 | 4.3x | |
| verify_rootfs_integrity | 181 | 1,000 | 5.5x | |
| **Stats collection** | | | | |
| stats_desktop | 24 | 1,000 | 41.7x | Rust inline, Cyrius heap walk |
| **Boot step marking** | | | | |
| mark_all_steps_complete | 5,770 | 33,000 | 5.7x | |
| **Runlevel switching** | | | | |
| plan_runlevel_switch | 1,298 | 5,000 | 3.9x | |
| **Crash action** | | | | |
| on_service_crash | — | 460 | — | Cyrius only |
| **Audit logging** | | | | |
| audit_log_record | — | 1,000 | — | Cyrius only (shim) |
| **Safe command** | | | | |
| safe_command_display | 146 | — | — | Not ported |
| **Execution plan** | | | | |
| boot_execution_plan | 3,378 | — | — | Not ported |
| boot_execution_plan_waves | 3,757 | — | — | Not ported |
| **API responses** | | | | |
| list_services_desktop | 536 | — | — | Not ported |
| system_status_desktop | 564 | — | — | Not ported |
| system_metrics_desktop | 211 | — | — | Not ported |
| boot_log_desktop | 185 | — | — | Not ported |
| **Resource limits** | | | | |
| resource_limits_prlimit | 772 | — | — | Not ported |

## Analysis

**Typical overhead: 4–8x vs Rust.** Expected for a language without LLVM optimizations,
inlining, or register allocation beyond R12 spill. The Cyrius compiler produces
unoptimized x86_64 — every variable is a stack slot, every function call goes through
the full calling convention.

**Outliers:**
- **stats_desktop (41.7x)**: Rust accesses struct fields inline. Cyrius walks a hashmap
  + vec for every stat. This is an algorithmic difference, not a compiler limitation.
- **boot_seq construction (18–31x)**: Vec/string allocation overhead. Rust reuses
  pre-allocated vectors; Cyrius allocates fresh each time via `alloc()`.
- **state_transitions (0.58x — Cyrius faster)**: Simple integer comparisons. Cyrius
  avoids Rust's match exhaustiveness overhead and enum discriminant checks.

**Where Cyrius is competitive (< 6x):**
- init_new_desktop: 4.7x
- plan_runlevel_switch: 3.9x
- configure_readonly_rootfs: 4.3x
- resolve_waves_wide_20: 5.2x
- resolve_order_desktop: 5.5x
- mark_all_steps_complete: 5.7x

These are the higher-level orchestration paths where allocation overhead
is amortized over real work.

## Rust-only benchmarks (not yet ported to Cyrius)

These require functions that haven't been ported:
- `boot_execution_plan` / `boot_execution_plan_waves` — execution plan builders
- `list_services` / `system_status` / `system_metrics` / `boot_log` — API response builders
- `resource_limits_prlimit_commands` — prlimit command generation
- `safe_command_display` — SafeCommand Display impl
