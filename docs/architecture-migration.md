# Architecture migration map

Accepted 2026-10-22. This is a responsibility/deletion map, not another phased
plan. [Implementation R0-R6](implementation.md#delivery-plan-and-gates) is the
only schedule; [domain architecture](domain-architecture.md) is the target.
The first unchanged build must work on the new format with obsolete paths removed.

## Responsibility map

| Existing responsibility | Destination | Retirement condition |
| --- | --- | --- |
| Hand-authored SSA JSON and opaque ID arithmetic | `.rule`, KDL and checked symbolic bindings in Rust Authoring | All live rules lower through the shared checker; generated canonical output reproduces |
| `tests/support::*::stage`, successor chains and per-packet publishing binaries | Production manifest-driven release builder and generic fixture harness | Clean clone builds the current endpoint without ignored/local predecessors |
| Release/catalog/recipe implementation inside Import | Authoring library/developer commands | Runtime Import and PoB decoder do not link release tooling |
| PoB source-program/callback/UI emulation in native Engine/Data | Delete; translate necessary game law to owned rules, necessary external syntax to Import, source execution to offline PoB | No native runtime dependency or data carrying Lua-shaped behavior remains |
| Legacy default features, embedded `game-data.json`, obsolete inspect/profile commands | Delete with last necessary consumer | Dependency/content checks confirm absence, including native/Wasm Import |
| Required Python exporters and boundary checks | Equivalent Rust workflow or deletion when obsolete | Supported build/data/test commands require no Python; useful fixtures retained |
| Rust source/Cargo.lock hashes as behavior identity | Semantic operation contracts and compiled artifact identity; provenance sidecar | Formatting/refactoring does not change behavioral identity |
| Repeated namespaces and String IDs in runtime values | Compact canonical form and checked package-local handles | One decoder/format, bounded load, typed-index tests and measurable hot-path improvement |
| Full release reproduce/clone on evaluation | Runtime-only load once; full reproduce in Authoring/CI | Evaluation uses immutable runtime data without authoring artifacts |
| Candidate constants in each freshly compiled plan | Dependency-aware structural plan + indexed parameters | Cold/warm, mutation invalidation, serial/parallel and fresh-finalist tests agree |
| Native execution in CLI glue; old search measurement mapping | Public native Evaluator and direct owned metric/objective binding | CLI/search share library behavior, availability and requirement assessment |
| Hundreds of test binaries and repeated support inclusions | Explicit shared modules/support crate and data-driven vectors | Meaningful behavior retained in a locally runnable, CI-covered suite |
| Optional actual PoB host and external build interchange | Retain as isolated reference/acquisition and Import boundaries | No native inner-loop fallback; exact source protocol and discrepancy reporting |

No new permanent `pob-compat` or `legacy-inspect` layer is adopted. Moving the old
interpreter into another crate without removing runtime dependence would not meet
the user's requested cleanup. A small necessary external parser is allowed only
as an explicit Import responsibility with behavioral tests.

## Cutover rules

- Migrate in coherent checkpoints; temporary offline conversion code is permitted
  only with its deletion task in R4. Normal loaders never accept both formats.
- Rebuild/reimport current artifacts. There are no shipped consumers requiring
  compatibility. Do not add aliases or preserve obsolete versions out of habit.
- Preserve established game-domain ownership and failure semantics while changing
  representation. The review does not authorize weakened coverage or defaults.
- Migrate valuable test assertions to the shared harness; delete exclusive
  scaffolding, historical publisher copies and stale IDs. An old test count or
  public command is not evidence that a feature must survive.
- The first complete new-format build and remaining originals are validation
  targets, not a reason to keep an old native runtime. Actual PoB remains the
  reference; evidence can outlive the code that generated it.
- Do not start further game-mechanic content until R4's single-path and passing-CI
  gate is met. UI remains later product work.

## Existing domain work that survives

Generic Actors, exact occurrences/providers/recipients, shared typed raw inputs,
source-property preparation, usage/scenario intent, explicit participation,
application stacking, contribution queries, skill-owned resource obligations,
separate socket configurations and same-run requirement assessment remain the
domain direction. Implemented fragments and unfinished integration are recorded
in the [current plan](implementation.md#follow-up-register), not inferred from
a proposal's age or version number.

Pending proposals that change authority or coverage still need a concrete decision
when required. DSL approval is not approval to silently turn unknown membership
into complete-empty, merge action-specific and actor-wide effects, or broaden
provider permissions.

## Historical evidence

The [pre-review migration document](https://github.com/Azaril/poe-optimizer/blob/fa49fcea32f0413cde5f5983e179c34e6c7acbdf/docs/architecture-migration.md)
records older D/T milestones. They are superseded by R0-R6. The
[retirement register](legacy-retirement.md) classifies remaining deletions and
source discrepancies; it does not schedule another implementation track.
