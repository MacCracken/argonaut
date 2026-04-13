# Standards Compliance

## NIST SP 800-53 Rev. 5

| Control | Name | Coverage | Detail |
|---------|------|----------|--------|
| SI-2 | Flaw Remediation | Partial | Cyrius build toolchain + AGNOS library audits detect known vulnerabilities |
| SI-7 | Software/Firmware Integrity | Partial | dm-verity rootfs verification, read-only rootfs |
| SI-10 | Input Validation | Full | SafeCommand, device path validation, hash validation |
| SI-16 | Memory Protection | Full | No direct syscalls in library, integer overflow guards, explicit allocation model |
| AU-2 | Event Logging | Full | Structured tracing on all service operations |
| AU-3 | Audit Record Content | Full | Events include timestamp, service, PID, exit code, signal, error |
| CM-5 | Access Restrictions for Change | Partial | Read-only rootfs, `noexec` on tmpfs |
| CM-7 | Least Functionality | Full | Boot mode selects minimal service set (Edge: 1, Recovery: 0) |
| CP-10 | System Recovery | Partial | Recovery boot mode, emergency shell, graceful degradation |
| SA-11 | Developer Testing | Full | CI: `cyrius test`, `cyrius bench`, build check, coverage |
| SC-13 | Cryptographic Protection | Partial | dm-verity SHA-256, LUKS2 encryption |
| SC-28 | Information at Rest | Partial | LUKS full-disk encryption for data partition |
| SR-4 | Provenance | Partial | All dependencies are path-based AGNOS libraries or Cyrius stdlib — no external package registry |

## NIST Cybersecurity Framework (CSF) 2.0

| Function | Category | Coverage |
|----------|----------|----------|
| Protect | PR.DS-1 Data-at-rest | LUKS encryption |
| Protect | PR.DS-6 Integrity | dm-verity rootfs verification |
| Protect | PR.AC-4 Access control | Read-only rootfs, `noexec`/`nosuid`, privilege enforcement |
| Protect | PR.IP-1 Baseline config | Typed `ArgonautConfig` with secure defaults |
| Protect | PR.IP-12 Vulnerability mgmt | Cyrius build toolchain + AGNOS library version management |
| Detect | DE.CM-4 Malicious code | dm-verity detects rootfs tampering |
| Detect | DE.AE-1 Anomaly baseline | Health history, watchdog timeouts, failure tracking |
| Respond | RS.MI-1 Containment | Restart limits, backoff, emergency shell |
| Recover | RC.RP-1 Recovery plan | Recovery boot mode, emergency shell, LUKS close |

## Common Criteria (ISO/IEC 15408)

| SFR | Name | Coverage |
|-----|------|----------|
| FPT_FLS.1 | Failure with preservation of secure state | Emergency shell on critical boot failure |
| FPT_RCV.1 | Manual recovery | Recovery boot mode with emergency shell |
| FPT_TST.1 | Self test | dm-verity integrity verification at boot |
| FDP_ACC.1 | Subset access control | Read-only rootfs with explicit writable overlay whitelist |
| FDP_RIP.1 | Residual information protection | LUKS close, filesystem sync, swap deactivation on shutdown |
| FAU_GEN.1 | Audit data generation | All lifecycle events logged with structured tracing |

## CIS Benchmark for Linux

| Control | Description | Coverage |
|---------|-------------|----------|
| 1.1.2-5 | `/tmp` with `noexec,nosuid` | Full — `configure_readonly_rootfs()` |
| 4.1 | Audit logging | Full — structured tracing on all operations |
| 6.1 | System file permissions | Partial — read-only rootfs prevents modification |

## DISA STIG

| STIG ID | Requirement | Coverage |
|---------|-------------|----------|
| V-230264 | Cryptographic integrity protection | dm-verity SHA-256 |
| V-230223 | FIPS-validated crypto for data at rest | LUKS2 (FIPS depends on kernel) |
| V-230333 | `/tmp` with `nosuid` | Full |
| V-230334 | `/tmp` with `noexec` | Full |
| V-230346 | Read-only filesystem when feasible | Full — rootfs remounted `ro` |
| V-230473 | Audit records for privileged functions | Full — process spawn, signal, lifecycle events |

## SLSA (Supply-chain Levels for Software Artifacts)

| Level | Requirement | Status |
|-------|-------------|--------|
| SLSA 1 | Documented build process | Full — CI workflow, cyrius.toml |
| SLSA 2 | Version-controlled source | Full — Git + GitHub |
| SLSA 2 | Dependency management | Full — path-based AGNOS library dependencies, version-locked in cyrius.toml |
| SLSA 3 | Build as code | Full — CI in version-controlled YAML |
| SLSA 3 | Ephemeral environment | Full — GitHub Actions runners |

## NIST SSDF (SP 800-218)

| Practice | Coverage | Detail |
|----------|----------|--------|
| PO.1 Security requirements | Full | CLAUDE.md documents policies |
| PS.2 Verify third-party components | Full | AGNOS-owned dependency stack — all libraries are first-party or path-based |
| PW.1 Design for security | Full | SafeCommand by design, secure defaults |
| PW.5 Secure coding practices | Full | No direct syscalls in library, input validation, Cyrius type system |
| PW.7 Code analysis | Full | Cyrius build check, coverage, benchmark regression |
| PW.8 Testing | Full | 607 assertions / 26 suites, CI on every push |
| RV.1 Vulnerability identification | Full | AGNOS library audits on every PR |
