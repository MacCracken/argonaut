# 003 — Cyrius 6.2.x stdlib resolution & allocator quirks

Non-obvious constraints introduced by the **cyrius 6.0.x → 6.2.11**
toolchain bump (1.8.3). All four bit us during the bump; none are
argonaut bugs, but each requires a specific consumer-side shape.

## 1. `json` / `bigint` folded into `bayan`

Cyrius 6.2.x no longer ships standalone `lib/json.cyr` or
`lib/bigint.cyr`. The consolidated **`bayan`** module bundles
base64 + json + bigint and re-exports the legacy `json_*` / `bigint_*`
shims (delegating to `bayan_*`).

- Manifest: list **`bayan`** in `[deps] stdlib`, not `json` / `bigint`.
- Self-contained tests/benches: include `lib/bayan.cyr` (not
  `lib/json.cyr` / `lib/bigint.cyr`).

A `cyrius deps` that errors `cannot read .../lib/json.cyr` is this.

## 2. libro is the single `dist/libro.cyr` bundle — never its sub-modules

Per libro's `DEPS-PATTERN.md`, the **only** distribution artifact is
`dist/libro.cyr` (→ `lib/libro.cyr`). libro's `src/` modules
(`error.cyr`, `hasher.cyr`, `entry.cyr`, `verify.cyr`, `query.cyr`,
`retention.cyr`, `chain.cyr`, `export.cyr`, plus the extended
`store`/`merkle`/`signing`/`anchoring`/`timestamping`/`proof`/
`kernel_audit` set) are **not** copied into `lib/`. Include
`lib/libro.cyr` once; do not hand-include the sub-modules. (The
pre-bundle include lists in the test/bench headers were stale and were
collapsed in 1.8.3.)

## 3. `thread_local` must be an explicit include ahead of sigil

sigil 3.7.x's banked crypto scratch (`cbank()` / `crypto_tls_main_init()`,
reachable from any audit-chain hash) calls
`thread_local_{init,get,set}`.

Under cyrius **6.0.x** the manifest `[deps] stdlib` list auto-pulled
`thread_local` unconditionally. Under **6.2.x** the auto-resolver only
compiles a stdlib module whose symbols are referenced by the **src**
graph — a module referenced *only* by a transitive git dep (sigil, via
libro) is silently skipped, even when listed in the manifest. The
symptom is a build-time `undefined function 'thread_local_init'` and a
runtime **SIGILL (exit 132)** the moment the audit-hash path runs.

Fix: include `lib/thread_local.cyr` **explicitly, before sigil** in every
compilation unit that reaches the audit chain — `src/main.cyr`,
`src/bench_main.cyr`, `tests/test_header.cyr`, and each standalone
audit/bench `.tcyr` / `.bcyr`. This is a deliberate exception to the
"no stdlib includes in src files" rule (CLAUDE.md): the manifest cannot
express it under 6.2.x.

## 4. Bench harness: `alloc_init()` is idempotent since 6.1.23

The benches (`src/bench_main.cyr`, `tests/bcyr/*.bcyr`) used a per-
iteration `alloc_reset(); alloc_init();` to free transient allocations.
That relied on the **pre-6.1.23** behavior where `alloc_init()`
re-`mmap`'d a *fresh* chunk on every call, isolating the per-iteration
churn from the long-lived bench bookkeeping (the `benches` vec + `bN`
timers) sitting in the first chunk.

Cyrius **6.1.23** made `alloc_init()` idempotent (a no-op once the heap
is up). Now `alloc_reset()` rewinds to the base chunk and the next
allocation overwrites the bookkeeping → **SIGSEGV (exit 139)**.

Fix (1.8.3): replace the idiom with a **heap high-water-mark rewind**.
The `_heap_ptr` global (from `lib/alloc.cyr`) is accessible in-unit;
capture `_bm = _heap_ptr` after each section's fixtures are allocated,
then `_heap_ptr = _bm` at the top of each timed iteration. This frees
the iteration's garbage while preserving the bookkeeping and bounds
memory. Sakshi span tracing (default stderr target, no level gate) is
silenced with a null emit hook (`sakshi_set_emit_hook(0)`) so it neither
floods bench output nor inflates the timed init sections.
