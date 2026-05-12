# Argonaut: extend `EdgeBootConfig` with deployment-specific paths + PCR baselines

**Filed:** 2026-05-11
**Reporter:** kybernet 1.2.0 (AGNOS PID 1 init system)
**Affected:** `src/types.cyr` — `EdgeBootConfig` struct + `edge_config_default()`
**Severity:** **P1** — kybernet's 1.2.x arc is **fully blocked** on this struct extension at the consumer-side. kybernet 1.2.0 shipped capability-detection-only edge boot with stubs in place of the real verify/unlock calls; 1.2.1 (real-device verify) cannot start until argonaut lands the deployment-path fields. P1 here is the consumer's rate-of-progress signal — argonaut's own work is unaffected, but kybernet's primary current-arc deliverable is sitting on it.
**Status:** open.

## Summary

kybernet 1.2.0 introduced edge-boot orchestration (`src/lib/edge_boot.cyr`) that gates on `EdgeBootConfig` and calls into agnosys-storage / agnosys-trust. The current struct shape (carried unchanged from argonaut 1.5.x and earlier):

```cyrius
struct EdgeBootConfig {
    readonly_rootfs;     # 1/0
    luks_enabled;        # 1/0
    tpm_attestation;     # 1/0
    max_boot_ms;
    pcr_bindings;        # Str ptr — e.g. "7+14" (PCR INDICES only, no expected values)
}
```

…carries the BEHAVIORAL FLAGS but none of the DEPLOYMENT-SPECIFIC INPUTS that the underlying agnosys primitives need:

- **`dmverity_verify(data_device, hash_device, root_hash)`** — needs three paths/values that vary per deployment image
- **`luks_open(config, key_ptr, key_len)`** — needs the LUKS device path + a key source (TPM-unsealed blob or initramfs passphrase prompt)
- **`tpm_verify_measured_boot(expected)`** — needs a `vec` of `tpm_pcr_value` entries (bank + index + hex hash) as the baseline to verify against. `pcr_bindings` carries the indices but not the hashes.

So kybernet 1.2.0's edge_boot.cyr SCAFFOLDS the orchestration (gate logic, capability detection, PCR read for measurement-logging) and STUBS the actual verify/unlock calls with `klog("…device-path config lands in 1.2.1; skipped")` placeholders. The pre-flight returns "continue" without the real verification happening.

## What kybernet 1.2.1 wants

```cyrius
struct EdgeBootConfig {
    # ── existing fields (keep for backward compat) ──
    readonly_rootfs;
    luks_enabled;
    tpm_attestation;
    max_boot_ms;
    pcr_bindings;            # Str — PCR indices to read (e.g. "7+14")

    # ── new fields ──

    # dm-verity rootfs verify
    data_device;             # Str — path to data partition (e.g. "/dev/sda2")
    hash_device;             # Str — path to hash tree (e.g. "/dev/sda3")
    root_hash;               # Str — 64-char hex sha256 root hash from build-time veritysetup

    # LUKS unlock
    luks_device;             # Str — path to encrypted partition (e.g. "/dev/sda4")
    luks_name;               # Str — dm-crypt mapper name (e.g. "kybernet-data")
    luks_mount_point;        # Str — where to mount the unlocked volume (e.g. "/var/lib/agnos")
    luks_key_source;         # enum: KEY_TPM_UNSEAL | KEY_PASSPHRASE_PROMPT | KEY_KEYFILE
    luks_key_path;           # Str — for KEY_TPM_UNSEAL: path to sealed blob; for KEY_KEYFILE: keyfile path

    # TPM PCR baseline — what the measured PCRs SHOULD be after a clean boot
    expected_pcrs;           # vec of tpm_pcr_value (bank + index + hex hash)
}
```

kybernet 1.2.1 + agnosys-{storage,trust} would then call:

```cyrius
fn edge_boot_run(config) {
    var ec = config_edge(config);
    # ... existing detection ...

    # 1.2.1 — real verify
    if (load64(ec) == 1) {  # readonly_rootfs
        var verified = dmverity_verify(
            edge_config_data_device(ec),
            edge_config_hash_device(ec),
            edge_config_root_hash(ec)
        );
        if (verified == 0) { return 0; }   # FATAL
    }

    if (load64(ec + 8) == 1) {  # luks_enabled
        var key_buf = alloc(64);
        var key_len = 0;
        if (edge_config_key_source(ec) == KEY_TPM_UNSEAL) {
            key_len = tpm_unseal(edge_config_key_path(ec), key_buf, 64);
        } # ... other key sources
        var luks_cfg = ...; # build from ec fields
        luks_open(luks_cfg, key_buf, key_len);
        luks_zeroize_key(key_buf, 64);
        luks_mount(...);
    }

    if (load64(ec + 16) == 1) {  # tpm_attestation
        var verify_res = tpm_verify_measured_boot(edge_config_expected_pcrs(ec));
        if (is_err_result(verify_res) == 1) { return 0; }  # FATAL
    }
}
```

## Suggested accessor names (mirrors existing `config_*` pattern)

```cyrius
fn edge_config_data_device(ec)     { return load64(ec + 40); }
fn edge_config_hash_device(ec)     { return load64(ec + 48); }
fn edge_config_root_hash(ec)       { return load64(ec + 56); }
fn edge_config_luks_device(ec)     { return load64(ec + 64); }
fn edge_config_luks_name(ec)       { return load64(ec + 72); }
fn edge_config_luks_mount(ec)      { return load64(ec + 80); }
fn edge_config_key_source(ec)      { return load64(ec + 88); }
fn edge_config_key_path(ec)        { return load64(ec + 96); }
fn edge_config_expected_pcrs(ec)   { return load64(ec + 104); }
```

The existing `edge_config_default()` should leave the new fields as **zero / null vec / empty Str** — deployments without verified-boot don't pay any cost (kybernet's gate is `readonly_rootfs == 1` / `luks_enabled == 1` / `tpm_attestation == 1`; zeroed fields short-circuit before the new accessors get called).

## JSON-config round-trip

argonaut's existing `argonaut_config_*_to_json` / `_from_json` pattern should be extended to serialize/deserialize the new fields. The `expected_pcrs` vec needs a JSON array shape like:

```json
{
  "edge_boot": {
    "readonly_rootfs": true,
    "luks_enabled": true,
    "tpm_attestation": true,
    "max_boot_ms": 3000,
    "pcr_bindings": "7+14",
    "data_device": "/dev/sda2",
    "hash_device": "/dev/sda3",
    "root_hash": "abc123...",
    "luks_device": "/dev/sda4",
    "luks_name": "kybernet-data",
    "luks_mount_point": "/var/lib/agnos",
    "luks_key_source": "tpm_unseal",
    "luks_key_path": "/etc/kybernet/luks.sealed",
    "expected_pcrs": [
      {"bank": "sha256", "index": 7, "value": "ff00..."},
      {"bank": "sha256", "index": 14, "value": "0011..."}
    ]
  }
}
```

## Severity rationale (P1 in kybernet's current arc)

- **Single blocker for the kybernet 1.2.x arc.** kybernet 1.2.0 shipped the orchestration scaffolding (gate logic, capability detection, PCR measurement, hard-prereq gating, max_boot_ms budget, status accessors) — everything except the actual `dmverity_verify` / `luks_open` / `tpm_verify_measured_boot` calls, which are stubbed pending the new fields. Once the struct extension lands, kybernet 1.2.1 is roughly a one-day cut. Without it, the 1.2.x arc stalls at "capability detection" with the actual verified-and-sealed-boot security property unenforced via kybernet's pre-flight path.
- **Argonaut-side scope is contained and additive.** New fields, new accessors, JSON round-trip extension. No API break — existing consumers see the same field offsets unchanged; new accessors return zeros for unconfigured deployments. No new agnosys deps required.
- **The P1 here is the consumer rate-of-progress signal.** Argonaut's own 1.6.4 / 1.7.0 work isn't blocked on anything; this filing is asking argonaut to prioritize the kybernet-facing extension over its other "Open — gated on external work" items in the current arc. Workarounds exist for production (run veritysetup/cryptsetup from an initramfs hook before handing off to kybernet, same as systemd-based stacks do today) — they just don't move kybernet's roadmap forward.

## Tracking on kybernet's side

Kybernet's [`docs/development/roadmap.md`](https://github.com/MacCracken/kybernet/blob/main/docs/development/roadmap.md) **v1.2.1** entry calls this out:

> Needs argonaut-side `EdgeBootConfig` extension first. Tracking in argonaut's roadmap.
>
> - [ ] argonaut: extend `EdgeBootConfig` with `data_device` / `hash_device` / `root_hash` / `luks_device` / `expected_pcrs` (vec of PCR baselines)

The companion kybernet-side work (which goes when this lands) is enumerated in the same roadmap entry.

## Adjacent

- argonaut 1.6.4 (next on argonaut's roadmap) is native aarch64 CI. Edge-boot extension is roadmap-orthogonal to that; could land in a 1.6.5 or 1.7.0 cut depending on argonaut's scheduling preference.
- No new agnosys deps required — the existing agnosys 1.2.5 `storage` + `trust` profiles already export everything the new struct surface enables. The work is entirely argonaut-internal.
- No backward-compat break — the new fields are additive. Existing consumers reading `EdgeBootConfig` see the same fields at the same offsets; new accessors return zeros for unconfigured deployments.

## Suggested argonaut roadmap entry

Promote into the **next-arc deliverable** alongside 1.6.4 native aarch64 CI (or as 1.6.5 immediately after, depending on scheduling preference) — not the "Open — gated on external work" section, which is for items waiting on external triggers. This one's trigger has fired: kybernet 1.2.0 is the consumer asking for it.

```markdown
## Next — v1.6.5 — EdgeBootConfig deployment-paths extension (kybernet P1)

kybernet 1.2.0 shipped edge-boot scaffolding gated on capability
detection; real-device verify and LUKS unlock are blocked on
per-deployment paths (data/hash devices, root hash, LUKS device +
key source) and an expected-PCR baseline vec landing in the struct.

- [ ] `EdgeBootConfig` struct extension: data_device, hash_device,
  root_hash, luks_device, luks_name, luks_mount_point, luks_key_source,
  luks_key_path, expected_pcrs
- [ ] Accessors (mirror existing `config_*` pattern)
- [ ] `edge_config_default()` leaves new fields zeroed
- [ ] `to_json` / `from_json` round-trip for the extended shape
- [ ] No agnosys / cyrius pin movement; argonaut-internal cut

See [`docs/development/issues/2026-05-11-edgebootconfig-deployment-paths.md`](issues/2026-05-11-edgebootconfig-deployment-paths.md).
Filed by kybernet 1.2.0; unblocks kybernet 1.2.1.
```
