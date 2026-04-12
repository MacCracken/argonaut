# cc3 Compiler Issues Blocking Full Libro Integration

## Issue 1: `undefined variable 'ptr'` (3.5.2+)

**Introduced:** cc3 3.5.2
**Present in:** 3.5.2, 3.6.0, 3.6.1
**Not present in:** 3.5.0
**Regression test:** `tests/tcyr/cc3_ptr_regression.tcyr`

Compiling argonaut's standard include chain (stdlib + sigil + libro core 7 modules + argonaut src) fails with:
```
error:9362: undefined variable 'ptr'
```

No source file in argonaut references a variable called `ptr`. The error is a compiler-internal symbol leak introduced between 3.5.0 and 3.5.2.

**Impact:** Argonaut cannot build on cc3 3.5.2+. Currently using cc3 3.5.0 (last known good).

## Issue 2: READFILE 512KB Cap (fixed in 3.5.1)

**Fixed in:** cc3 3.5.1
**Regression test:** `tests/tcyr/cc3_readfile_cap.tcyr`

Two READFILE calls in `lex.cyr` capped per-include reads at 512KB. Fixed in 3.5.1.

## Issue 3: Token Limit 131072 (3.5.1+)

**Present in:** 3.5.1, 3.5.2, 3.6.0, 3.6.1
**Regression test:** `tests/tcyr/cc3_readfile_cap.tcyr`

Including all 19 libro modules exceeds the 131072 token limit. The READFILE cap is fixed but the expanded source is too large for the token table. Blocks including libro's signing, merkle, anchoring, timestamping, proof, stores, export, review, streaming modules.

## Summary

| cc3 version | 7 libro modules | 19 libro modules |
|-------------|-----------------|-------------------|
| 3.5.0       | PASS            | FAIL (READFILE)   |
| 3.5.1       | PASS            | FAIL (token limit)|
| 3.5.2       | FAIL (ptr)      | FAIL (ptr)        |
| 3.6.0       | FAIL (ptr)      | FAIL (ptr)        |
| 3.6.1       | FAIL (ptr)      | FAIL (ptr)        |
| 3.6.1+fix   | PASS            | FAIL (codebuf)    |
| 3.6.2       | PASS            | PASS (16 modules) / SEGV (19 — file_store/patra_store need lock fns) |

## Status (3.6.2)

Compiler limits (READFILE, codebuf, token) all resolved. 16 of 19 libro modules compile and run.
The remaining 3 (`file_store.cyr`, `patra_store.cyr`, `streaming.cyr`) reference `file_append_locked`, `file_lock_shared`, `file_unlock` which are not in argonaut's stdlib. These are patra/fs extended functions — adding `lib/patra.cyr` would resolve this.
