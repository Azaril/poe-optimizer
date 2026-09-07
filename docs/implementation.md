# Implementation log and resume point

Last updated: 2026-09-07

Current phase: M1 remains in progress. Backend-neutral evaluation contracts, typed metrics,
explicit options and coverage are implemented. Two independent Spark reference cases pass
local CLI parity; broader mechanic and mutation coverage remains open. Native Rust calculation
translation is active in parallel. All 78 local Windows tests, formatting, Clippy and portable
core/native WASM compilation pass. Windows/Linux CI also passes for `7dd2a09`. No optimizer
or worker pool is implemented.

This is the living record of delivery order, implemented behavior, validation, unresolved
work, and the next session's starting point. [Design](design.md) defines the intended system
and acceptance criteria; [execution and interfaces](execution-and-interfaces.md) and the
[calculation boundary decision](calculation-boundary.md) define
runtime and application contracts. The [PoB investigation](pob-integration.md) and
[prior-art review](prior-art-and-product-review.md) retain dated evidence and rationale.
Update this document at progress checkpoints, rather than adding implementation status to
the design documents.

## Resume here

1. Inspect `git status --short --branch` before editing. Code checkpoint `7dd2a09` is
   published with shared evaluation contracts, typed CLI output, options/coverage,
   independent calibration and the first native kernels. Read the validation table below.
2. Local validation is complete: 78 Windows tests pass, including exact minion selection
   and observed cooldown behavior. Formatting, Clippy and portable core/native WASM checks
   also pass. Windows/Linux CI, including both portable library checks, passes for this code.
3. Keep the PoB submodule at `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4` and unmodified.
   No Lua syntax overlay is required. Preserve original user fixtures and the independently
   generated calibration goldens; never refresh expected values using the Rust evaluator.
4. Expand M1 with a small attack/item/support interaction fixture, then controlled legal
   mutations across passive, item, support, supporting-skill, class and ascendancy choices.
   Include simultaneous skill/item locks and coupled changes. The three unresolved supplied
   entries are classified with related candidates; they are not automatically repaired.
   Investigate the pinned tree's dangling class-start connections before cross-class search.
5. Continue the native track with a bounded modifier aggregation/data-model slice and actual
   upstream differential evidence. The six numeric helpers are not a build evaluator and
   are not yet wired as a full `CalculationBackend`. Keep production dependencies free of
   Lua and OS services; the portable compilation pass is not a browser execution result.
6. Add candidate/lock/scoring models and tiny exhaustive joint-search fixtures through the
   shared engine boundary as M2 starts. Pooling, cancellation, caching and persistent workers
   remain later implementation work gated by correctness and shared resource accounting.
7. Replace this resume point and record evidence at the next checkpoint.

No product-scope answer blocks these tasks. Exact objectives, thresholds, required skill/item
subsets, and encounter assumptions remain explicit per-run inputs. The two Spark cases establish
independent host/extractor parity for shared PoB calculations, not independent certification of
PoE mechanics or complete evaluator coverage.

## Current repository and capabilities

- Branch: `main`; remote `origin` is `https://github.com/Azaril/poe-optimizer.git`
  ([repository](https://github.com/Azaril/poe-optimizer)). The user authorized publishing the
  project and supplied builds. Evaluator baseline `71c0af7` passed
  [hosted CI](https://github.com/Azaril/poe-optimizer/actions/runs/34147041293); current changes
  are published as `7dd2a09` with complete local evidence below.
- [Workspace](../Cargo.toml): Rust 2024 / minimum 1.93; shared application contracts, PoB
  import/runtime/supervision, a static native UTF-8 library, native calculation kernels,
  and the root CLI. The [core](../crates/poe-optimizer-core/src/evaluation.rs) now separates
  `CalculationBackend` from `EvaluationEngine`. Requests/results carry build documents,
  options, capabilities, typed measurements, coverage and versioned identity without exposing
  Lua or process APIs. The current `Engine` is synchronous. Contract validation rejects
  missing/duplicate/undeclared measurements, bad units, invalid finite metadata, and changed
  backend/request identity. Backends enforce their own deadline and return typed failures.
- [CLI](../src/main.rs): `import`, `evaluate`, and `metrics`. Evaluation uses report schema 2
  with a shared typed `evaluation` object; the private JSONL worker protocol is version 2.
  `--options <json>` selects one-based groups/actions and applies a named encounter's enemy
  level, boss kind and explicit five-component incoming hit. Repeatable `--metric` filters
  typed actor queries; `--raw` includes the backend diagnostic snapshot attachment. JSON/XML
  destinations must be new paths. Planner JSON conversion is not implemented.
- [Metric catalog](../crates/poe-optimizer-pob/src/metrics.rs): 15 definitions, 17 actor queries.
  Player metrics cover life/mana/energy shield, four capped resistances, PoB EHP and five
  maximum-hit values; selected hit DPS and average hit also support the selected minion.
  Units and `Finite` / `NonFinite` / `Unavailable` are explicit. `CombinedDPS` and `FullDPS`
  remain raw diagnostics, not catalog objectives. Broader action semantics, including the
  selected minion's cooldown-adjusted rate, remain diagnostic rather than rotation guarantees.
- [Options/context](../crates/poe-optimizer-core/src/options.rs) record requested selection,
  encounter overrides and effective MAIN inputs/conditions. [Coverage](skill-coverage.md)
  records resolved/unresolved entries, ownership and provenance, Full DPS membership/counts,
  selected action context, and dangling tree connections without silently repairing imports.
- [Importer](../crates/poe-optimizer-pob/src/import.rs): exact UTF-8 XML and hash preservation;
  1 MiB share-code and 8 MiB XML limits, complete zlib-stream validation, 100,000-node
  parser cap, and no DTD/custom entity expansion. Predefined/numeric XML escapes work.
- [PoB backend](../crates/poe-optimizer-pob/src/backend.rs) implements the shared contract using
  a supervised fresh process. [Runtime](../crates/poe-optimizer-pob/src/runtime.rs) embeds
  `mlua 0.12.1`, `luajit-src 210.7.3+1ee778a` (LuaJIT `2.1.1787165859`) and static
  `luautf8 0.1.6`. Production does not load upstream Windows DLLs. The
  [native-module notes](../crates/poe-optimizer-lua-utf8/README.md) preserve exact source/header
  provenance. The supervisor enforces startup-inclusive deadlines, bounded I/O, request
  identity, retained stderr and parent-owned scratch cleanup after kill/reap.
- [Evaluator preflight](../crates/poe-optimizer-pob/src/preflight.rs) rejects missing/ambiguous
  Build/Tree identity and unsupported versions; container-only import remains separate.
  Post-load guards reject stale/incomplete calculations. Pure-Rust source verification and
  adapter/native fingerprints avoid Git subprocesses inside workers.
- [Independent calibration](calibration-reference.md): two small level-1 Spark cases, normal
  mapping and Pinnacle bossing, each have 19 recorded numeric outputs. A separate Windows
  C/Lua host runs the same pinned PoB through its bundled LuaJIT `2.1.1784580905`; it shares
  neither production host nor extractor code. [CLI parity](../tests/calibration.rs) passes
  locally against those committed goldens. This validates the limited host/extractor path,
  not independently modeled mechanics or realistic endgame quality.
- [Native engine](native-engine.md): `poe-optimizer-engine` contains six resolved-input
  numerical functions for player/monster hit chance, deflection, fractional/rounded armour
  reduction, and rounding. Production has no external dependencies, Lua, I/O or scheduling.
  Five differential/property tests against actual pinned Lua functions and a
  `wasm32-unknown-unknown` library compile pass. It is not a full build backend, browser app,
  measured speedup or replacement for the current PoB evaluator.
- Objective parsing, candidate mutation/legality, search, Rayon pools, cancellation, reports,
  recovery and GUI are **not implemented**. [objective.toml](../examples/objective.toml)
  remains design notation, not accepted CLI input.
- [CI](../.github/workflows/rust.yml) initializes pinned submodules and checks workspace
  formatting, Clippy and tests on Windows/Linux, plus portable core/native WASM compilation.
  Both hosted jobs pass for `7dd2a09`, including tests and portable core/native compilation.

## Delivery plan and gates

Milestone IDs are stable references for review notes. A milestone is complete only when its
acceptance evidence is recorded below. M1 and M2 may overlap after the evaluator interface
is concrete; real-build recommendations depend on both.

| Milestone | Status | Deliverable and acceptance gate |
| --- | --- | --- |
| M0: bootstrap and alignment | Complete | Rust scaffold, pinned submodule, end-state design, source/prior-art review, preserved source fixture, and this implementation record. Scaffold validation passes; this does not establish evaluator correctness. |
| M1: evaluator and fixtures | In progress; shared boundary/typed options and limited independent Spark parity implemented; broader mechanic and legal mutation coverage outstanding | Reproducible mlua/LuaJIT startup in a Rust worker, bounded import, supervised fresh-process evaluation, typed metric mappings, controlled mutations across all six dimensions, export/re-import parity, and isolation evidence. Complete the checklist below. |
| M2: reusable core and synthetic joint search | Shared evaluation interfaces available; candidate/scoring/search work not started | Candidate/domain/lock models, metric policies, coupled mutations and repair, Rayon/job APIs, shared budgets and events. Match exhaustive tiny domains, escape coordinated-change traps, preserve multiple locks, and verify deterministic one/many-worker results plus cancellation/dedup/accounting. |
| M3: first usable joint optimizer | Not started | Integrate all six dimensions with multicore PoB evaluation; ship preflight, evaluation/comparison, search, saved-result reranking, recovery, verified exports, and offline reports. Meet the end-to-end gates below. |
| M4: broader catalogs and upgrade workflows | Not started | Extend mechanic/equipment/skill coverage and conditional upgrade/bundle ranking with explicit inventory, cost, and comparison semantics. Retain parity and lock guarantees. |
| M5: richer objective policies | Not started | Unit-checked expressions, composite and ordered priorities, soft preferences, Pareto selection, and explicit robust aggregation. Test policy-specific selection and preserve hard constraints. |
| Desktop GUI | Deferred until CLI/report contracts stabilize | Choose frontend; Tauri is a candidate. Reuse core jobs, results and comparison models. Verify CLI/GUI parity, responsive cancellation and native-worker packaging. Does not depend on finishing every M4/M5 feature. |
| Native Rust calculation replacement | Active in parallel | Six numerical kernels and portable library compilation pass focused checks. Expand through modifier/data and complete pipeline slices with differential parity before exposing a native build backend. Performance and browser execution remain unmeasured. |
| PoE1 adapter | Later, separate track | Add a distinct versioned rules/data/evaluator adapter after PoE2 interfaces are proven; do not mix game identities or reuse PoE2 parity claims. |

Narrow passive/item/skill experiments are internal validation steps. The first usable release
must search classes, ascendancies, passives, equipment, support gems, and supporting active
skills together within explicit finite catalogs. It must support 1..N required skills and
1..N exact equipped item instances, independently of other locks.

### M1 checklist: establish the calculation oracle

- [x] Pin and inspect upstream loading, calculation, mutation, and export seams.
- [x] Decode the supplied PoB export without altering source bytes; record provenance,
      game/tree identity, and structural checks.
- [x] **M1.1 Runtime — implemented and validated on Windows/Linux.**
      Embedded LuaJIT and static UTF-8 boot the pinned wrapper and load the supplied build.
      Host callbacks supply paths, time, logging, noninteractive failures and controlled
      scratch writes. Dynamic native loading is disabled; each VM stays in its worker.
      No pin change or syntax overlay was needed. Source-manifest reproducibility,
      mismatch rejection and native provenance checks pass, along with hosted Windows/Linux
      startup, native and fixture tests. Exact dependency/source identities are recorded.
- [ ] **M1.2 Boundary — implemented and locally validated; scenario coverage remains limited.** Shared
      `CalculationBackend` / `EvaluationEngine` separate application contracts from hosting.
      CLI schema 2 and protocol 2 carry explicit skill/scenario options, versioned identity,
      typed measurements, coverage and optional diagnostic attachments. Eleven core contract
      tests, seven options/coverage integration tests and full workspace checks pass.
      Part/stat-set overrides and a complete resolved scenario model remain unimplemented.
      The synchronous engine has no worker pool or preemptive native execution boundary.
- [ ] **M1.3 Import and metrics — container/preflight complete; semantics partial.**
      Typed units/availability now cover 15 definitions and 17 actor queries. Coverage
      classifies the three unresolved supplied entries and exposes selected ownership,
      Full DPS membership, manual/generated provenance and missing tree connections.
      No automatic repair is performed. Extend selected minion command/usage-model coverage
      and expand action/mechanic coverage before treating measurements as search objectives.
- [ ] **M1.4 Baseline parity — partial, with independent calibration evidence.**
      Two self-cast Spark scenarios have independently hosted/extracted, same-pin PoB
      reference outputs and explicit bossing/mapping assumptions. Both pass the production
      CLI comparison of all 19 recorded values plus typed units/context/coverage. Reference
      generation uses different host, extractor and LuaJIT builds; the calculation engine
      is shared, so this is not independent game-mechanics certification. Expand to attack,
      support/equipment, minion and survival cases. Same-adapter A/B/A/export comparisons
      remain useful separate evidence; cached export values are not a reference oracle.

- [ ] **M1.5 Mutation parity — not started across the required dimensions.** Exercise legal
      passive, item, support, supporting-skill, class and ascendancy changes separately
      and jointly. Preserve locks and verify export, point/resource accounting and effect
      removal against fresh materialization/reload. Include at least two required skills
      and two exact equipped-item locks; the level-only integration change is insufficient.
- [ ] **M1.6 Isolation and failures — partial.** Parser/protocol failures, blocked-stdin
      timeout/reaping, scratch cleanup, fresh-worker A/B/A and export/reimport checks pass.
      Complete broader order, worker-count and failure coverage. Persistent reset/reload
      workers, pools and cancellation are not implemented; enable reuse only after matching
      the fresh-process reference.
- [ ] **M1.7 Measurements — partial observation only.** The recorded standalone debug CLI
      request took 2,603.4796 ms inside the evaluator and 2,996.6718 ms wall time, including
      source hashing and startup, on the hardware below. Filesystem-cache state was not
      controlled; this single observation is not a throughput benchmark. Measure repeated
      startup/evaluation/export/reload costs, memory and error rate; test warmed/reset and
      marginal paths only after correctness is established, with required EHP enabled.

The planner JSON export is preserved as auxiliary input/provenance. Raw PoB XML and PoB
share codes are runnable import paths; planner conversion follows only when its format
and missing semantics can be mapped faithfully.

### M2 and M3 acceptance details

M2 begins with one user-selected registered metric to maximize/minimize and a conjunction
of typed constraints, including strict and inclusive comparisons. Reject unknown metrics,
invalid units, contradictory bounds, non-finite thresholds and unsupported policies.
The engine consumes a scoring policy independently of game metric names; M5 extends the
policy implementations without redesigning candidate search.

M2 synthetic fixtures must include tiny exhaustively checked joint domains and a case where
every single-change improvement path stalls but a coordinated move wins. Exercise class
and ascendancy legality, tree connectivity/budgets, equipment multiplicity/slots, skill
compatibility/resources, and lock-preserving repair. Keep legal infeasible exploration
separate from verified feasible incumbents.

M3 must demonstrate:

- All six dimensions searchable in one real PoB-backed run, including cross-class
  alternatives. No silent freezing of a requested dimension or removal of a required lock.
- Complete-candidate verification and export/re-import checks independent of optimization
  cache/fast paths, with reserved evaluation/time budget and honest incomplete status.
- Shared CPU/memory limits across Rayon and Lua workers; bounded queues, in-flight
  deduplication, cancellation, capped retries and consistent attempt accounting.
- Preflight coverage/assumptions, common-baseline comparisons, grouped changes, constraint
  shortfalls, useful distinct alternatives, JSON run artifacts and offline HTML reports.
  Saved-result reranking is explicitly distinguished from a new search.
- Atomic recovery checkpoints with actual verification status. Evaluated-only recovered
  seeds are freshly validated before trusted reuse; a warm start is a new run/budget.
  Exact random/search-state continuation remains separate later work.
- Quality against random, greedy and alternating-domain baselines under equal evaluation
  budgets and several seeds. Measure fixed-candidate scaling and end-to-end quality at
  5, 15 and 30 minutes on recorded hardware, in explicit bossing and mapping contexts.
  Report each case, failures and variability; mapping uses documented proxies.
  There are no measured throughput, quality, or scaling results yet.

## Fixture ledger and technical unknowns

[Fixture README](../tests/fixtures/builds/README.md) and
[metadata](../tests/fixtures/builds/pobarchives-Dfz36mCq.metadata.json) preserve the full
source/hash record. The immutable decoded
[XML](../tests/fixtures/builds/pobarchives-Dfz36mCq.xml) is 49,642 bytes with SHA-256
`e3c0d0b40fa682260a1713acb03d52d720f4b769ac91b0501cbe2a84dc468194`.

The source is level 96 Sorceress / Disciple of Varashta, `PathOfBuilding2` root, tree
`0_5`. All 130 allocated IDs exist in the pinned tree. There are 19 skill groups and
16 referenced item records (13 equipment references and three tree jewels).
The planner title's patch 0.5.5 is a source claim; XML `targetVersion="0_1"` is not
independent evidence of the game patch.

| Unknown / limitation | Evidence so far | Next check |
| --- | --- | --- |
| Runtime portability | The selected embedded LuaJIT accepts `count += 1` and the leading `#@` lines; the pinned wrapper boots on Windows without vendor changes. | Hosted Windows/Linux checks pass; retain version/source pins on future updates. |
| Native ABI and source identity | Static `luautf8 0.1.6` links to mlua's vendored LuaJIT. Native/Unicode tests, eight vendored-file provenance checks, source-manifest reproducibility and mismatch rejection pass locally. | Hosted portability checks pass. Repeat provenance checks deliberately when updating pinned inputs. |
| Selected minion metric semantics | Main player action is `SummonSandDjinnPlayer`; selected minion action is `ExplosiveTeleportSandDjinn`, owned by group 1/gem 1. Typed non-finite chaos immunity is represented explicitly. Upstream initially declares average mode, then clears it after applying the self-cast cooldown: final `show_average=false`. The typed value is PoB's selected-action rate, not a rotation guarantee. | Add command/clone/usage-model coverage; do not present raw ~60,384.684 as independently validated sustained minion DPS. Add independent minion/action references. |
| Full DPS membership | Structured coverage shows no included supplied-build groups and raw player `FullDPS` zero. Spark calibration includes one group and compares its raw roll-up. | Keep membership/counts/contributions explicit; Combined DPS and Full DPS are not catalog objectives. Independent Spark agreement does not validate multi-skill uptime or aggregation. |
| Unresolved and granted skills | Three entries remain unresolved. Powered Zealot matches two Spectre monsters; Navira's Well and Kelari's Deception match minion command actions. Coverage distinguishes 15 manual, three tree-granted and one item-granted groups. | Preserve ambiguity and original input; exact relationships are hints, not repair or legality. Exercise supported action changes and coupled support/granted-skill effects without omission or double-counting. |
| Host behavior and diagnostics | Fourteen startup messages correspond to dangling connections in tree `0_5`; coverage exposes actual missing targets and allocated endpoints. The Sorceress start and supplied allocated nodes are unaffected by those specific edges. | Their impact on other classes remains unverified. Compare topology against corrected upstream/game data before enabling cross-class tree search. Preserve stderr and coverage together. |
| Legality, freshness and parity | Fresh-load guards and A/B/A/round trips pass at the previous checkpoint; two new Spark scenarios pass independent-host CLI parity. Neither provides full build legality or six-dimension mutation evidence. | Complete legal/coupled mutation and failure/isolation coverage before reuse, fast paths or real-build recommendations. |

These are technical unknowns, not requests for the user to guess internal PoB IDs.
Ask for user input when a reproduced issue forces a product trade-off or the supplied
material cannot establish an intended build choice; present the concrete alternatives then.

## Validation evidence

### Current contracts, calibration and native checkpoint — local checks passed

Recorded on 2026-09-07 on Windows/x64 for published code checkpoint `7dd2a09`.
Preserve the historical results below as evidence for their own commits. Local validation
is complete; hosted results are recorded below.
The final Lua edit before the standalone smoke was a comment documenting cooldown behavior;
formatting/Clippy and the smoke were rerun after it.

| Check | Result / limit |
| --- | --- |
| Core shared contract suite, `cargo test -p poe-optimizer-core --test engine_contract --locked` | **11 passed.** Fake alternate/boxed backends exercise custom catalogs, complete actor queries, explicit unavailable/non-finite values, unit/schema checks, capability rejection, identity/options echo, JSON-safe metadata and typed timeout/failure propagation. |
| Independent CLI calibration, `cargo test --test calibration --locked` | **1 passed, covering both Spark scenarios.** Each fresh CLI process matches all 19 independent raw values, source/fixture identity, typed finite units, absent-minion availability, effective encounter and selected Spark coverage. Absolute tolerance `1e-8`, relative tolerance `1e-9`; applied as `max(absolute, abs(reference) * relative)`. |
| Standalone reference generation, `pwsh -File scripts/reference-calibrate.ps1` | Separate MSVC C driver plus bundled DLLs produced the two committed reference JSONs. Two independent fresh runs per fixture reproduced all 19 metrics exactly. Runtime/compiler/source/fixture/generator hashes and assumptions are recorded in [the reference notes](calibration-reference.md). No Rust evaluator generated expected values. |
| Native numerical parity, `cargo test -p poe-optimizer-engine --locked` | **5 passed.** Six functions compare against actual pinned upstream Lua using source hashes, boundary grids, interpreted/warmed execution, exceptional-value classification and positive-domain properties. The crate remains a resolved-input numerical slice, not a full evaluator. |
| Portable core and native libraries, `cargo check -p poe-optimizer-core -p poe-optimizer-engine --lib --target wasm32-unknown-unknown --locked` | Passed. Production library has no Lua/C/OS dependency. No browser runtime, bindings, scheduler or browser numerical/performance test has run. |
| Options/coverage integration suite | **7 passed.** Exact group/player/minion selection, mapping overrides, capped resistance units, invalid selectors/metrics/fields, nonfinite Config numbers, DamageOverTime/incoming-hit mismatch, unresolved/provenance/tree coverage and classified infinity. MAIN action ownership is checked against the authoritative actor list, not PoB's mutable UI display list. The minion's final average flag is correctly false after cooldown adjustment. |
| `cargo fmt --all -- --check` | Passed after all code edits. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed with warnings denied. |
| `cargo test --workspace --all-targets --locked` | **78 passed**, zero failures; one ignored subprocess helper is explicitly invoked by the deadline test. Includes the original fixture A/B/A and export/reimport regressions, 11 core contract tests, 5 native parity tests, and 5 new preflight tests. |
| Sources, artifacts and documentation | Read-only audit matched all 1,082 source entries and all 16 available reference artifact hashes. Both independent runs match each golden's 19 values exactly. Original user fixtures retain exact hashes/lengths, submodule remains clean; README/docs relative file links and `git diff --check` pass. |
| Publish and hosted Windows/Linux CI | **Passed for `7dd2a09` on both Windows and Linux:** formatting, Clippy, full tests, and core/native WASM compilation. [Hosted run 34150698841](https://github.com/Azaril/poe-optimizer/actions/runs/34150698841). The subsequent living-document update changes documentation only. |
| Standalone CLI smoke | `runs/m1-contracts-20260907-dd6640a0.json` and `.xml` (ignored) contain schema 2, 17 typed measurements, exact selected SandDjinn action and raw diagnostic attachment. Adapter fingerprint `bf6a6c2f134812e9f738a1f0c4581472a5130d239a85d48e1a169e16c261e292`. This debug run took 2,582.5707 ms inside evaluation, including source verification/startup; one observation is not a benchmark. `metrics` emits 15 definitions without launching Lua. |
| Optimization, native speedup/scaling and browser execution | Not implemented or measured. No throughput, quality, scaling or browser-runtime claim follows from parity and compilation. |

The PoB source remains `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`, source-manifest hash
`8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675`.
Current production hosting is mlua `0.12.1` / LuaJIT `2.1.1787165859` / static UTF-8
`0.1.6`; the independent reference uses bundled LuaJIT `2.1.1784580905` on x64. The
Spark comparisons establish host/extractor parity for those shared PoB calculations.
Native kernel differential checks translate the pinned implementation; neither kind
of evidence independently certifies game mechanics.

### Historical M1 evaluator checkpoint — published and validated

Recorded on 2026-09-07 on Windows/x64 with Rust 1.93. Evaluator commit
[`71c0af7`](https://github.com/Azaril/poe-optimizer/commit/71c0af7a8813776036b4e9750f4ac713bb52a832)
is published; both hosted Windows/Linux jobs passed.

| Check | Result / limit |
| --- | --- |
| `cargo fmt --all -- --check` | Passed. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed. |
| `cargo test --workspace --all-targets --locked` | **47 passed**: 13 importer, 9 preflight, 6 source, 2 runtime, 11 supervisor, 2 native and 4 CLI tests. One ignored subprocess fixture is explicitly invoked by the timeout test. |
| Fresh-worker isolation and export | A/B/A changes level 96 → 95 → 96. Export/reimport compares all finite actor metrics using `1e-8 * max(abs(expected), 1)` tolerance, alongside actor/summary checks. This is same-adapter evidence. |
| Container/evaluator separation | Empty `PathOfBuilding2` imports as a valid container but evaluation rejects it. Unsupported target versions reject before returning default/startup metrics. |
| Output safety and supervision | Output aliases reject before evaluation; bounded I/O, blocked-stdin deadline/kill/reap and scratch cleanup tests pass. |
| Sources and native provenance | Original source/decoded fixture hashes unchanged; pinned submodule clean. All eight vendored native/header file identities match provenance. Source manifest reproduces and wrong-pin rejection passes. |
| Hosted CI | [Run 34147041293](https://github.com/Azaril/poe-optimizer/actions/runs/34147041293) passed on both `windows-latest` and `ubuntu-latest`: checkout, Rust setup, formatting, Clippy and workspace tests. |
| Independent PoB reference / joint mutation parity | Not established; independent calibration and all-dimension mutation coverage remain M1 work. |

The standalone debug observation used `mlua 0.12.1`, LuaJIT `2.1.1787165859`
(`luajit-src 210.7.3+1ee778a`), and static `luautf8 0.1.6`, on an AMD Ryzen 9 9950X3D
with 16 physical cores, 32 logical processors and approximately 64 GiB RAM. Evaluator
elapsed time was **2,603.4796 ms**; CLI wall time was **2,996.6718 ms**, including source
hashing, bootstrap, calculation and export. Filesystem-cache state was uncontrolled;
this is one observation, not a cold-disk, throughput or scaling benchmark.

The sample retained raw minion `TotalDPS = 60384.68419307677`, energy shield `9403`,
player `FullDPS = 0`, three unresolved-skill warnings, explicit non-finite output keys,
and 429 bytes of diagnostics. Agreement with source-cached DPS is not independent parity.
Local ignored artifacts: `runs/m1-evaluator-20260907-b9f5b961.json` and
`runs/m1-evaluator-20260907-b9f5b961.xml`; these are not committed fixtures.

Recorded artifact identities:

- Source hash: `8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675`.
- Adapter hash: `250468f42ebc774348a986bf573fb7bcff211fd16862ad9e03026f61563945fd`.

The earlier 25-test pass predates review changes and is superseded by the 47-test result.

### Historical scaffold baseline

Recorded on 2026-09-07 for baseline `72bf6a7` and its scaffold ancestors:

| Check | Result / limit |
| --- | --- |
| `cargo fmt --all -- --check` | Passed. |
| `cargo clippy --all-targets --locked -- -D warnings` | Passed on Windows with Rust 1.93.0. |
| `cargo test --all-targets --locked` | Passed, **zero tests**; verifies only the starter target builds. |
| CLI help/version/invalid argument | Checked; invalid arguments exit with code 2. |
| Fixture decoding and structure | Source/output hashes checked; bounded complete zlib decode and XML parsing; 130 tree IDs checked. This was an investigation, not an implemented importer test suite. |
| Submodule and documentation | Recorded pin clean; local document links and `git diff --check` passed at the prior checkpoint. |
| Lua evaluation, mutation/export parity, search/scaling | Not run at this historical scaffold baseline. |
| Hosted CI | Workflow configured; no hosted run result recorded at this checkpoint. |

The earlier living-document checkpoint changed documentation only. Local file/heading links and
git whitespace checks were validated; fixture bytes and the submodule pin were preserved.
Rust checks were not repeated because code, manifests and workflow were unchanged.

For each implemented change, record the appropriate validation and actual outcome here or
link a committed result summary. Do not count future checklists as evidence. Generated
benchmark artifacts belong under ignored `runs/`; commit concise reproducible summaries
and fixture/config identities when they become available.

## Checkpoint protocol

At session start, read the resume point and inspect the working tree. At each coherent
feature completion, runtime experiment that changes direction, and before session handoff:

1. Update milestone/task status to match the code and observed behavior.
2. Record commands, outcomes, runtime/data versions and relevant evidence paths. Preserve
   failures and unresolved questions; distinguish untested hypotheses from reproduced blockers.
3. Replace the resume point with the next small executable task and any prerequisites.
   Note unfinished files or local-only setup that the next session must preserve.
4. Update the design only if scope, architecture, invariants or public contracts changed.
   Keep agreed end-state capabilities even while implementation is incomplete. Update
   integration evidence if a new pin or experiment changes an earlier finding.
5. Keep README capability claims accurate, review the diff, and commit a coherent checkpoint
   when appropriate. Record the commit on the next update or identify the checkpoint by
   its dated label; a document cannot contain its own final Git hash.

### Checkpoint history

| Date | Reference | Outcome |
| --- | --- | --- |
| 2026-09-07 | `fbfe216` | Initialized Rust scaffold, PoB submodule and initial design. |
| 2026-09-07 | `7926b60` | Clarified configurable objectives and extensible scoring. |
| 2026-09-07 | `c278da5` | Designed multicore execution and reusable CLI/GUI boundaries. |
| 2026-09-07 | `72bf6a7` | Reviewed WoW prior art; confirmed joint scope and cross-class search; preserved/decoded supplied minion fixture. |
| 2026-09-07 | `8eb2e23` | Separated end-state design from delivery tracking; recorded M1 resume point and evidence; connected the user-created GitHub repository. |
| 2026-09-07 | `46473e3` | Recorded explicit user authorization to push the project and supplied fixtures to GitHub; cleared the pending publishing question. |
| 2026-09-07 | `1be92d7` | Preferred mlua hosting inside isolated Rust evaluator workers. |
| 2026-09-07 | `71c0af7` | Added bounded import, evaluator preflight, embedded/native hosting, source verification, fresh-worker supervision and CLI evaluation/export. Formatting, Clippy, 47 local tests and hosted Windows/Linux CI pass; independent parity and optimization remain outstanding. |
| 2026-09-07 | `7dd2a09` | Added backend-neutral contracts, CLI schema/protocol 2, typed catalog/options/coverage, two independent Spark scenarios and the first six native numerical functions. All 78 local tests, formatting, Clippy and core/native WASM compilation pass. Both hosted Windows/Linux jobs pass in run `34150698841`; broader M1 coverage remains open. |

### Hosting decision checkpoint

On 2026-09-07 the user preferred `mlua` for Lua hosting/interaction where feasible. The
design and M1 resume point now prioritize a Rust worker embedding LuaJIT. Direct LuaJIT
execution is a diagnostic/reference fallback. Process isolation, shared budgets, fresh
verification and all joint-search requirements remain in force.

Official [mlua 0.12.1 feature metadata](https://raw.githubusercontent.com/mlua-rs/mlua/v0.12.1/Cargo.toml)
confirms `luajit` and `vendored` support and Rust 1.88 as its declared minimum, below this
project's Rust 1.93 minimum. At that decision checkpoint this was source-level feasibility evidence, not a successful
local build, ABI validation or PoB calculation; subsequent results are recorded above. Read-only inspection also identified the
Windows native-module dependency and bootstrap requirements recorded in the
[integration notes](pob-integration.md#embedding-bootstrap-checks). Pin the dependency/runtime selection
when implementing M1. No Cargo dependencies or runtime code changed at this checkpoint.

## Decisions still deferred

No answer is needed before M1. Choose the project's distribution license before a public
release; confirm the desktop framework and packaging before GUI work; choose acquisition
data sources before trade/upgrade ingestion. Benchmark-specific metrics and usage profiles
must be documented when those fixtures are made runnable, without turning their choices
into mandatory goals for all users.
