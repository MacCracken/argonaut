# 004 — libro 2.8.0 thin sigil & cyrius 6.4.x quirks

Non-obvious constraints introduced by the **cyrius 6.2.11 → 6.4.62**
toolchain bump plus the **libro 2.7.4 → 2.8.0** dep bump (1.8.4). All three
bit us during the bump; none are argonaut bugs, but each requires a specific
consumer-side shape.

## 1. Test/bench files must NOT `include "lib/sigil.cyr"`

libro 2.8.0 resolves a **thin sigil surface** — the capability sub-bundles
`lib/sigil_sha256.cyr`, `lib/sigil_sha_ni.cyr`, `lib/sigil-mldsa.cyr`, and
`lib/sigil_hex.cyr` (sha256 + ed25519 + ML-DSA + hex) — instead of the
monolithic `dist/sigil.cyr`. The monolith inlines the x509 / RSA /
authenticode path, whose bignum tables carry a **~13 MB static `.bss`**
footprint that the audit chain never uses (per libro's own manifest note, it
balloons `build/libro` from ~1 MB to ~14 MB).

`src/main.cyr` gets the thin surface automatically because the manifest
`[deps.libro]` auto-resolution prepends libro's transitive sigil. But a
self-contained `.tcyr` / `.bcyr` that hand-includes `lib/sigil.cyr` pulls the
**monolith** (cyrius materializes it on demand from the dep), which:

- drags the 13 MB static into the test binary (build warns *large static
  data (13049792 bytes)*), and
- collides with the manifest-resolved thin bundle — the same `sig_alg_name`
  / `hash_alg_name` / … land twice, emitting **hundreds of `duplicate fn …
  (last definition wins)`** warnings (234 per file in the audit suites).

None of argonaut's test/bench files call `sigil_*` directly — sigil is only
reached transitively through libro's audit-chain hashing/signing. So the
fix (1.8.4) is simply to **drop `include "lib/sigil.cyr"`** from every file
that carries it — **ten** in all:

- 6 self-contained suites: `audit_a`, `audit_b`, `audit_lifecycle`,
  `cc3_ptr_regression`, `cc3_readfile_cap`, `parity`;
- both benches: `tests/bcyr/argonaut.bcyr`, `tests/bcyr/api.bcyr`;
- the **shared `tests/test_header.cyr`**, included by ~21 header-driven
  suites (miss this and those 21 keep emitting 234 dups each); and
- the **bench-gate entry `src/bench_main.cyr`**, which
  `scripts/bench-history.sh` builds — with the monolith it produced a
  **14.3 MB** bench binary (vs ~791 KB thin) whose last-wins duplicated
  crypto made the recorded bench numbers an unfaithful proxy for the
  production `main.cyr` binary.

Let the manifest resolve the thin bundle instead. Removing the include drops
the per-file `duplicate fn` count 234 → 8 (the residual 8 are longstanding
`sakshi_*` manifest-resolve dups, unrelated) and eliminates the large-static
warning. A grep for the include must cover `src/`, `tests/tcyr/`,
`tests/bcyr/`, **and shared headers** — scoping it to the two test globs is
how the header + bench entry were missed on the first pass.

A test that legitimately needs a sigil primitive should include the specific
thin sub-bundle (e.g. `lib/sigil_sha256.cyr`), never the monolith.

## 2. cyrius 6.4.x reachability is a hard error, not a warning

Under 6.2.x, a call to an undefined function that DCE could not prove
unreachable produced a *warning* and the binary still emitted. Under
**6.4.62** the linker **refuses to emit a binary with N reachable undefined
function(s)** (downgrade only with `--allow-undef`).

This surfaced three audit suites (`audit_lifecycle`, `parity`,
`cc3_ptr_regression`) that included `src/health.cyr` and `src/init.cyr`
(which call `resolve_host_ipv4` / `write_ipv4_octets` in `src/resolver.cyr`
and `audit_log_*_persistent` / `pal_chain` in `src/audit_ext.cyr`) but
omitted those two modules from their include headers. Under 6.2.x the calls
were treated as unreachable (warnings); 6.4.62 flags them reachable → build
fails.

Rule of thumb: a self-contained test that includes `src/init.cyr` or
`src/health.cyr` must also include `src/resolver.cyr` (before `health`) and
`src/audit_ext.cyr` (after `audit`), matching `src/main.cyr`'s order.

## 3. Bench output is decimal with mixed units (ns/us/ms)

cyrius 6.4.x's `lib/bench.cyr` prints timings as **decimals in
auto-scaled units** — `2.389us`, `min=908ns`, `max=10.200ms` — where older
toolchains emitted integer microseconds (`4us`). `scripts/bench-history.sh`'s
parser matched only the old integer-`Nus` shape, so after the bump it
silently appended **zero rows** while still printing *"Results appended"*.

Fix (1.8.4): the parser normalizes every token to microseconds
(ns ÷ 1000, us ×1, ms ×1000, s ×10⁶) with 3-decimal precision. From 1.8.4
`bench-history.csv` stores decimal µs; the historical integer rows remain
numerically comparable (a pre-6.4.x `1` and a 6.4.x `1.4` differ mostly by
the rounding the old format threw away — worth remembering when reading
sub-µs "regressions" on 1 µs-scale micros).
