# cc3 READFILE 512KB Cap

**Discovered:** 2026-04-11 during libro 1.0.2 integration
**Compiler:** cc3 3.5.0
**Status:** Waiting on cc3 3.5.0 release — may already be fixed

## Problem

When argonaut includes all 19 libro modules (~600KB expanded source), cc3 silently truncates the source, producing misleading parse errors:
- `expected '=', got fn`
- `expected '}', got end of file`

Only 7 of 19 libro modules can be included. The remaining 12 (signing, merkle, anchoring, timestamping, proof, stores, export, review, streaming, kernel_audit, file_store, patra_store) are in `lib/libro/` but excluded from `src/main.cyr`.

A **nested include variant** also triggers the bug at lower total sizes — including libs through a header file (e.g. `tests/test_header.cyr`) consumes read budget that inner includes need.

## Reproduction

```sh
# Current 7-module build works (~470KB expanded)
cat src/main.cyr | cc3 2>/dev/null > build/argonaut  # OK

# Adding remaining 12 libro modules pushes past 512KB — fails
# Add these to src/main.cyr after lib/libro/chain.cyr:
#   include "lib/libro/store.cyr"
#   include "lib/libro/export.cyr"
#   include "lib/libro/review.cyr"
#   include "lib/libro/merkle.cyr"
#   include "lib/libro/signing.cyr"
#   include "lib/libro/anchoring.cyr"
#   include "lib/libro/timestamping.cyr"
#   include "lib/libro/proof.cyr"
#   include "lib/libro/kernel_audit.cyr"
#   include "lib/libro/file_store.cyr"
#   include "lib/libro/patra_store.cyr"
#   include "lib/libro/streaming.cyr"
```

## Suspected root cause

Two READFILE calls in `src/frontend/lex.cyr` (lines ~1061, ~1209) cap reads at `524288 - op` (512KB) while the preprocess buffer is 1MB.

## Impact on argonaut

- Binary includes only libro core chain (7 modules) — no signing, merkle, persistence, export
- Audit tests inline their includes directly instead of using shared headers
- `parity.tcyr` inlines includes for the same reason
