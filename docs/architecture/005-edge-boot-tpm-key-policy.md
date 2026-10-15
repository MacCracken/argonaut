# 005 — Edge boot: the TPM key-release policy belongs to the enroller

**Status:** documented constraint, 1.16.2 (the user's decision, 2026-10-14: document, no
code change). It comes from the 2026-10-14 audit's deferred item for the CVE-2026-32606
class. (Numbers 003 and 004 were toolchain notes, deleted at 1.8.5; the series is not
renumbered.) The edge-boot model it refines is
[ADR-004](../adrs/004-edge-boot-security-model.md).

## What argonaut does, and does not do

`unlock_luks` (`src/edge_boot.cyr`) builds one command:

```
cryptsetup open --type luks2 --token-only --token-id 0 --tries 1 -- <device> <name>
```

- **`--token-only`:** the volume key comes only from LUKS2 token 0, which is normally a
  TPM2-sealed key enrolled offline. There is no passphrase fallback, so someone at the
  console cannot type their own recovery key.
- **`--tries 1`:** one attempt, then failure.
- **`execute_edge_boot`** runs read-only rootfs, then dm-verity, then LUKS, in that order.
  A verity failure stops the boot before the unlock.

argonaut has **no TPM code**. It neither enrols nor seals keys, and it never reads or
extends a PCR. Which measurements release the key is decided when the token is enrolled.
kybernet links sigil's TPM bundle (`dist/sigil-tpm.cyr`: `tpm_read_pcr`,
`tpm_extend_pcr`, `tpm_seal`, `tpm_unseal`) and reads the PCRs named by
`EdgeBootConfig.pcr_bindings`. Its default, `"7+14"`, is a **read-out set**: kybernet
compares it against `expected_pcrs` and reports drift to dmesg. It is not a sealing
policy, and changing it does not change what the TPM releases.

## The threat (CVE-2026-32606 class)

IncusOS, March 2026: the LUKS key was sealed to PCR 7 (Secure Boot state) and PCR 11 (the
signed kernel image). Both are identical for **any** root filesystem that boots under the
same signed kernel and initramfs. So an attacker with physical access can swap the root
partition for one of their own:
1. The firmware and kernel measure exactly as before.
2. The TPM releases the volume key.
3. The attacker's userspace runs with the data partition unlocked.

Two gaps let that through:
- **Nothing binds the release to the identity of the root filesystem.**
- **Nothing stops a second unseal later in the same boot.** Code that runs after the
  unlock can ask the TPM again, because the PCRs still hold their pre-unlock values.

## What an AGNOS enrolment must do

This is work for kybernet and the provisioning tooling, not for argonaut:

1. **Measure the root filesystem into a PCR before unlocking.** After dm-verity has
   verified the root hash, and before `unlock_luks` runs, extend that verified root hash
   into a PCR the token is bound to. PCR 15 is the conventional "system identity" PCR:
   `tpm_extend_pcr(TPM_SHA256, 15, <sha256 of the verity root hash>)`. Enrol the token to
   **PCR 7 + PCR 15**, so a swapped root, which has a different verity root hash, does not
   reproduce PCR 15 and gets no key.
2. **Cap the PCR after unlocking.** As soon as `unlock_luks` succeeds, extend PCR 15 again
   with a fixed "unlocked" marker. From then until reboot no state reproduces the enrolled
   value, so code running after the unlock, including the emergency shell, cannot unseal
   the key a second time.
3. **Prefer a signed (authorized) policy to raw PCR values**, so an update that changes the
   root hash re-signs the policy instead of re-enrolling every board.
4. **Keep the emergency shell authenticated.** From 1.16.2, `emergency_shell_default()`
   requires authentication with an Argon2id credential. Without step 2, a root shell
   reached by making a boot step fail still sees PCR values that unseal the key. Step 2
   closes that path for the key; authentication closes it for everything else.

## Why argonaut does not do this itself

- **The dependency:** extending a PCR needs a TPM interface, which means sigil's TPM bundle
  (`/dev/tpmrm0` command marshalling, or `tpm2-tools`). That is a dependency argonaut does
  not carry, and kybernet already does.
- **The ordering:** the measurement has to sit between kybernet's own dm-verity
  verification and the unlock, and only kybernet knows the root hash it verified.
  argonaut's `execute_edge_boot` takes device paths and the hash as parameters and has no
  provisioning state.

If argonaut ever owns edge boot end to end, the shape above is the specification: measure
the verified root hash into PCR 15, unlock, then cap PCR 15.

## Verification

None in argonaut: there is no TPM code to test. The kybernet-side steps need real hardware
(a TPM2 and a LUKS2 TPM token), which kybernet's roadmap tracks under "Blocked on hardware".
The roadmap's Kybernet integration section records this note as an open item.
