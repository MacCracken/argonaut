# ADR-004: Edge Boot Security Model

**Status**: Accepted
**Date**: 2026-04-02

## Context

Edge devices (IoT, NUC, RPi) operate in physically untrusted environments. An attacker with physical access could modify the rootfs, tamper with the data partition, or extract encryption keys. The boot chain must verify integrity before trusting the root filesystem.

## Decision

Edge boot enforces a layered security model:

1. **Read-only rootfs**: Root partition is remounted `ro`. Writable state goes to tmpfs overlays (`/tmp`, `/var/run`, `/var/log`, `/var/tmp`) with size caps and `noexec`/`nosuid` flags.

2. **dm-verity integrity**: The rootfs is verified against a SHA-256 root hash using `veritysetup`. If verification fails, boot halts — the system does not start services on a tampered rootfs.

3. **LUKS2 encryption**: Data partition is encrypted with LUKS2. Unlock uses TPM2-backed tokens (`cryptsetup open --token-only`) — no passphrase in edge mode.

4. **Input validation**: All device paths are validated (prefix, character whitelist, no `..` traversal). All hash values are validated (length, hex characters). This prevents injection through configuration.

5. **Boot time budget**: `EdgeBootConfig.max_boot_time_ms` enforces a deadline. Exceeding it indicates a potential tamper or hardware issue and is reported in `EdgeBootResult`.

## Consequences

- **Positive**: Tampered rootfs is detected before any services start.
- **Positive**: Data at rest is encrypted — physical disk extraction yields nothing.
- **Positive**: Read-only rootfs prevents persistent malware installation.
- **Negative**: TPM2 dependency means edge devices need TPM hardware (or fallback to passphrase, which defeats unattended boot).
- **Negative**: dm-verity requires a separate hash partition and rebuild on every rootfs update.

## Alternatives Considered

- **fs-verity**: Per-file verification rather than whole-partition. More granular but doesn't protect against missing files or directory structure tampering.
- **IMA/EVM**: Linux Integrity Measurement Architecture. More complex, requires kernel support and policy management. Better for audit than prevention.
- **No encryption**: Simpler but unacceptable for edge devices in physically untrusted locations.
