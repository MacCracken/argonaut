# Examples

Working code against argonaut's `src/`. Run each from the repository root; stdlib and
dependencies come from `cyrius.cyml`. CI runs every example, so each one builds and runs
against the current source.

| Example | Shows | Run |
|---|---|---|
| [`boot-plan.cyr`](boot-plan.cyr) | Startup waves and order from service dependencies, and a refused cycle. Spawns nothing. | `cyrius run docs/examples/boot-plan.cyr` |

Each example opens with a comment saying why it exists. The
[quickstart guide](../guides/quickstart.md) covers wiring argonaut into a consumer, and the
supervisor loop.
