# Contributing to Argonaut

Thank you for your interest in contributing to Argonaut.

## Development Workflow

1. Fork and clone the repository
2. Create a feature branch from `main`
3. Make your changes
4. Build: `cyrius build src/main.cyr build/argonaut`
5. Test: `cyrius test`
6. Benchmark: `cyrius bench`
7. Open a pull request

## Prerequisites

- [Cyrius](https://github.com/MacCracken/cyrius) toolchain (cc3 + cyrius) v3.2.5+
- Linux x86_64

## Build Commands

| Command | Description |
|---------|-------------|
| `cyrius build src/main.cyr build/argonaut` | Compile |
| `cyrius test` | Run all .tcyr test suites |
| `cyrius bench` | Run all .bcyr benchmarks |
| `cyrius check src/main.cyr` | Syntax check |

## Project Structure

```
src/           Application modules (.cyr)
tests/tcyr/    Test suites (.tcyr) — auto-discovered by cyrius test
tests/bcyr/    Benchmarks (.bcyr) — auto-discovered by cyrius bench
tests/         test_header.cyr (shared includes + helpers)
lib/           Cyrius stdlib (local copy)
docs/          Architecture, roadmap, ADRs
rust-old/      Original Rust source (archived)
```

## Adding a Module

1. Create `src/module_name.cyr`
2. Prefix all functions with `modulename_` to avoid namespace collisions
3. Add `include "src/module_name.cyr"` to `src/main.cyr`
4. Add tests in `tests/tcyr/module_name.tcyr`
5. Update CHANGELOG.md

## Cyrius Conventions

- Everything is i64 — use `sizeof(StructName)` for allocation sizes
- Heap allocation via `alloc()` from `lib/alloc.cyr`
- Memory access via `store64/load64` (and 8/16/32 variants)
- Strings are null-terminated C strings
- Error returns: -1 for error, 0+ for success
- Comments with `#`
- No `match` as variable name (reserved keyword in cc3)
- Max 6 function parameters

## Code Style

- One module per `.cyr` file
- Module-level comment block at top
- Function names: `prefix_verb_noun()` (e.g., `init_start_service()`, `audit_log_record()`)
- Constants via `enum` blocks
- SafeCommand for all process execution (prevent shell injection)
- No `unwrap()` patterns — propagate errors
