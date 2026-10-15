# Standards Compliance

How argonaut's controls map to common frameworks. The ratings follow
[`docs/security/posture.md`](../security/posture.md):
- **Full** means argonaut implements the control itself.
- **Consumer** means kybernet implements it through argonaut's pre-exec hook
  ([ADR-013](../adrs/013-policy-enforcement-in-the-consumer.md)).
- **Edge only** means it applies on the edge-boot path.

## NIST SP 800-53 Rev. 5

| Control | Name | Coverage | Detail |
|---------|------|----------|--------|
| SI-2 | Flaw Remediation | Partial | Periodic security audits (`docs/audit/`), a security-hardening pass and known-CVE sweep before every release, hash-locked dependencies |
| SI-7 | Software / Firmware Integrity | Partial (edge only) | dm-verity rootfs verification, read-only rootfs |
| SI-10 | Input Validation | Full | SafeCommand; device-path, hash, mapper-name, PID-file, notify-datagram, port and URL validation |
| SI-16 | Memory Protection | Partial | Explicit allocation model; integer-overflow guards; no libc |
| AU-2 | Event Logging | Full | Every service lifecycle decision goes into the audit chain; health results on change, plus an hourly heartbeat |
| AU-3 | Audit Record Content | Partial | Each record has a timestamp, severity, source, action and details (the service name). It carries no pid, exit code or signal |
| AU-9 | Protection of Audit Information | Partial | SHA-256 hash-linked chain; optional signed snapshots and merkle proofs. It streams by default (not retained in memory) |
| CM-5 | Access Restrictions for Change | Partial (edge only) | Read-only rootfs; `noexec` on `/tmp` and `/var/tmp` |
| CM-7 | Least Functionality | Full | The boot mode selects a minimal service set (Edge: 1, Recovery: 0) |
| CP-10 | System Recovery | Partial | Recovery boot mode; an authenticated emergency shell |
| AC-6 | Least Privilege | Consumer | Per-service capabilities, uid / gid, Landlock and seccomp, applied by the consumer's pre-exec hook. Without a hook, services run as root |
| SA-11 | Developer Testing | Full | A `.tcyr` regression test for every behaviour change; every suite runs in CI on each push; aarch64 under qemu-user; a qemu PID-1 harness run at each release. No coverage measurement |
| SC-13 | Cryptographic Protection | Partial | SHA-256 audit chain; Argon2id emergency credential; dm-verity SHA-256 and LUKS2 (edge) |
| SC-28 | Information at Rest | Partial (edge only) | LUKS2 encryption of the data partition |
| SR-4 | Provenance | Partial | Git dependencies pinned by tag and hash-locked in `cyrius.lock`; no external package registry; no provenance attestation |

## NIST Cybersecurity Framework (CSF) 2.0

| Function | Category | Coverage |
|----------|----------|----------|
| Protect | PR.DS-1 Data at rest | LUKS2 (edge) |
| Protect | PR.DS-6 Integrity | dm-verity rootfs verification (edge); hash-linked audit chain |
| Protect | PR.AC-4 Access control | Read-only rootfs and `noexec` scratch dirs (edge); emergency-shell authentication; notify sender authentication; per-service privilege via the consumer's hook |
| Protect | PR.IP-1 Baseline config | `ArgonautConfig` with secure defaults: authentication required, streaming audit, bounded timeouts |
| Protect | PR.IP-12 Vulnerability management | Pre-release security hardening and CVE sweep; hash-locked dependencies |
| Detect | DE.CM-4 Malicious code | dm-verity detects rootfs tampering (edge) |
| Detect | DE.AE-1 Anomaly baseline | Health history, watchdog timeouts, failure tracking |
| Respond | RS.MI-1 Containment | Restart limits and window, backoff, watchdog kills, give-up audited |
| Recover | RC.RP-1 Recovery plan | Recovery boot mode, emergency shell |

## Common Criteria (ISO/IEC 15408)

| SFR | Name | Coverage |
|-----|------|----------|
| FPT_FLS.1 | Failure with preservation of secure state | Emergency shell on a failed required boot step (authenticated); a failed pre-exec hook refuses the service |
| FPT_RCV.1 | Manual recovery | Recovery boot mode with emergency shell |
| FPT_TST.1 | Self test | dm-verity verification at boot (edge); the qemu PID-1 harness (development) |
| FDP_ACC.1 | Subset access control | Read-only rootfs with an explicit writable-overlay list (edge) |
| FDP_RIP.1 | Residual information protection | Partial: `sync` before poweroff. Unmount, swapoff and LUKS close are planned steps (`init_plan_shutdown`) that the consumer executes |
| FAU_GEN.1 | Audit data generation | Lifecycle events in the hash-linked audit chain; structured tracing |

## CIS Benchmark for Linux

| Control | Description | Coverage |
|---------|-------------|----------|
| 1.1.2-5 | `/tmp` with `noexec,nosuid` | Edge only (`configure_readonly_rootfs`) |
| 4.1 | Audit logging | Partial: service lifecycle only |
| 6.1 | System file permissions | Partial: the read-only rootfs prevents modification (edge) |

## DISA STIG

| STIG ID | Requirement | Coverage |
|---------|-------------|----------|
| V-230264 | Cryptographic integrity protection | dm-verity SHA-256 (edge) |
| V-230223 | FIPS-validated crypto for data at rest | LUKS2 (FIPS depends on the kernel) |
| V-230333 | `/tmp` with `nosuid` | Edge only |
| V-230334 | `/tmp` with `noexec` | Edge only |
| V-230346 | Read-only filesystem when feasible | Edge only: rootfs remounted `ro` |
| V-230473 | Audit records for privileged functions | Partial: service start / stop / kill / restart; no per-signal records |

## SLSA (Supply-chain Levels for Software Artifacts)

| Level | Requirement | Status |
|-------|-------------|--------|
| SLSA 1 | Documented build process | Met: CI and release workflows, `cyrius.cyml` |
| SLSA 2 | Version-controlled source | Met: Git + GitHub |
| SLSA 2 | Hosted build | Met: GitHub Actions |
| SLSA 2 | Signed provenance | **Not met**: releases publish `SHA256SUMS` only |
| SLSA 3 | Hardened, ephemeral build | Partial: ephemeral runners; no provenance, so level 3 is not claimed |

## NIST SSDF (SP 800-218)

| Practice | Coverage | Detail |
|----------|----------|--------|
| PO.1 Security requirements | Full | CLAUDE.md: hard constraints, the security-hardening checklist |
| PS.2 Verify third-party components | Full | AGNOS-owned dependencies, pinned by tag and hash-locked (`cyrius deps --verify` in CI) |
| PW.1 Design for security | Full | SafeCommand, fail-closed defaults, a policy hook that fails closed |
| PW.5 Secure coding practices | Full | Input validation, no shell, arch-dispatched syscalls, constant-time secret comparison |
| PW.7 Code analysis | Partial | `cyrius lint` / `fmt --check` / `vet` and a pattern scan in CI; periodic manual audits. No coverage measurement |
| PW.8 Testing | Full | A regression test for every behaviour change, written to fail first for MEDIUM+ findings; every suite in CI on each push (counts in `docs/development/state.md`) |
| RV.1 Vulnerability identification | Partial | A security-hardening pass and known-CVE sweep before every release; dated audit reports |
