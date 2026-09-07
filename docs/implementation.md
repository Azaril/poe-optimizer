# Implementation log and resume point

Last updated: 2026-09-07

Current phase: M1 evaluator implementation is in progress. Import and fresh-process PoB
evaluation pass local Windows checks. Hosted CI and independent calculation parity remain
outstanding. No optimizer is implemented.

This is the living record of delivery order, implemented behavior, validation, unresolved
work, and the next session's starting point. [Design](design.md) defines the intended system
and acceptance criteria; [execution and interfaces](execution-and-interfaces.md) defines
runtime and application contracts. The [PoB investigation](pob-integration.md) and
[prior-art review](prior-art-and-product-review.md) retain dated evidence and rationale.
Update this document at progress checkpoints, rather than adding implementation status to
the design documents.

## Resume here

1. Inspect `git status --short --branch` before editing. Local Windows formatting, Clippy
   and all 47 tests pass for this evaluator checkpoint. Commit/push the reviewed changes,
   then inspect and record hosted Windows/Linux CI results; no hosted result is recorded yet.
2. Keep the PoB submodule at `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4` and unmodified.
   The pinned source boots; neither `+=` nor leading `#@` lines require an overlay with
   the selected LuaJIT. Do not repeat the original compatibility spike.
3. Next establish **typed metric and skill coverage plus independent calibration parity**.
   Resolve the three ambiguous fixture entries, selected minion action, Full DPS membership
   and granted/manual provenance. Add a small attack/self-cast calibration fixture and
   explicit bossing/mapping assumptions; obtain independently reproduced PoB outputs.
4. Exercise legal mutations and coupled interactions across all six dimensions before
   calling M1 complete or enabling persistent workers/search recommendations. The passing
   level-change A/B/A and same-adapter export/reimport checks are a starting point, not
   independent reference or complete mutation-parity evidence.
5. Replace this resume point and record evidence at the next checkpoint.

No product-scope answer blocks these tasks. Exact objectives, thresholds, required skill/item
subsets, and encounter assumptions remain explicit per-run inputs. A successful calculation
or agreement with cached source values does not establish independent evaluator parity.

## Current repository and capabilities

- Branch: `main`; remote `origin` is `https://github.com/Azaril/poe-optimizer.git`
  ([repository](https://github.com/Azaril/poe-optimizer)). The user explicitly authorized
  publishing the project and supplied build files. Last published baseline: `1be92d7`;
  this locally validated evaluator checkpoint is ready for commit/push and hosted CI.
- [Workspace](../Cargo.toml): Rust 2024 / minimum 1.93; core protocol/snapshot types,
  PoB importer/runtime/supervisor, a static native UTF-8 library, and the root CLI.
- [CLI](../src/main.rs): `import <input>` validates raw XML or a PoB share code;
  `evaluate <input>` runs a fresh worker and emits experimental JSON with optional XML
  export. Output files are created without overwriting existing files. Planner JSON
  conversion is not implemented.
- [Importer](../crates/poe-optimizer-pob/src/import.rs): exact UTF-8 XML and hash preservation;
  1 MiB share-code and 8 MiB XML limits, complete zlib-stream validation, 100,000-node
  parser cap, and no DTD/custom entity expansion. Predefined/numeric XML escapes work.
- [Runtime](../crates/poe-optimizer-pob/src/runtime.rs): `mlua 0.12.1` hosts vendored
  `luajit-src 210.7.3+1ee778a` (reported LuaJIT `2.1.1787165859`) and statically linked
  `luautf8 0.1.6`. The [native-module notes](../crates/poe-optimizer-lua-utf8/README.md)
  record source/header provenance and the shared runtime linkage; bundled Windows DLLs
  are not loaded. The unmodified pinned wrapper calculates and exports the supplied build.
- [Supervisor](../crates/poe-optimizer-pob/src/supervisor.rs): versioned JSONL with request
  identity, one fresh process per evaluation, startup-inclusive deadline, bounded pipe I/O,
  structured failures, and retained stderr even on success. Parent-owned scratch storage
  is removed after kill/reap and I/O cleanup; timeout and cleanup tests pass.
- [Evaluator preflight](../crates/poe-optimizer-pob/src/preflight.rs) rejects missing or
  ambiguous Build/Tree identity and unsupported format versions; it does not change the
  container-only importer contract or certify build legality. Post-load guards reject
  incomplete/stale calculations. Pure-Rust source verification and native/adapter
  fingerprints pass their checks without spawning Git from workers.
- Snapshots expose raw actor outputs, explicit non-finite keys and coverage warnings.
  Certified metric mappings, objective parsing, search, Rayon pools, cancellation, reports,
  recovery and GUI are **not implemented**. [objective.toml](../examples/objective.toml)
  remains design notation, not accepted CLI input.
- [CI](../.github/workflows/rust.yml) now checks out submodules and runs workspace formatting,
  Clippy and tests on Windows/Linux, building the embedded native dependencies. Hosted
  results for this checkpoint have not been recorded.

## Delivery plan and gates

Milestone IDs are stable references for review notes. A milestone is complete only when its
acceptance evidence is recorded below. M1 and M2 may overlap after the evaluator interface
is concrete; real-build recommendations depend on both.

| Milestone | Status | Deliverable and acceptance gate |
| --- | --- | --- |
| M0: bootstrap and alignment | Complete | Rust scaffold, pinned submodule, end-state design, source/prior-art review, preserved source fixture, and this implementation record. Scaffold validation passes; this does not establish evaluator correctness. |
| M1: evaluator and fixtures | In progress; runtime/import/fresh-worker path validated locally; independent parity and broader coverage outstanding | Reproducible mlua/LuaJIT startup in a Rust worker, bounded import, supervised fresh-process evaluation, typed metric mappings, controlled mutations across all six dimensions, export/re-import parity, and isolation evidence. Complete the checklist below. |
| M2: reusable core and synthetic joint search | Not started | Candidate/domain/lock models, metric policies, coupled mutations and repair, Rayon/job APIs, shared budgets and events. Match exhaustive tiny domains, escape coordinated-change traps, preserve multiple locks, and verify deterministic one/many-worker results plus cancellation/dedup/accounting. |
| M3: first usable joint optimizer | Not started | Integrate all six dimensions with multicore PoB evaluation; ship preflight, evaluation/comparison, search, saved-result reranking, recovery, verified exports, and offline reports. Meet the end-to-end gates below. |
| M4: broader catalogs and upgrade workflows | Not started | Extend mechanic/equipment/skill coverage and conditional upgrade/bundle ranking with explicit inventory, cost, and comparison semantics. Retain parity and lock guarantees. |
| M5: richer objective policies | Not started | Unit-checked expressions, composite and ordered priorities, soft preferences, Pareto selection, and explicit robust aggregation. Test policy-specific selection and preserve hard constraints. |
| Desktop GUI | Deferred until CLI/report contracts stabilize | Choose frontend; Tauri is a candidate. Reuse core jobs, results and comparison models. Verify CLI/GUI parity, responsive cancellation and native-worker packaging. Does not depend on finishing every M4/M5 feature. |
| Rust calculation migration / PoE1 | Deferred | Profile before porting calculations; require differential parity. Add PoE1 as a separate versioned rules/evaluator adapter after PoE2 interfaces are proven. |

Narrow passive/item/skill experiments are internal validation steps. The first usable release
must search classes, ascendancies, passives, equipment, support gems, and supporting active
skills together within explicit finite catalogs. It must support 1..N required skills and
1..N exact equipped item instances, independently of other locks.

### M1 checklist: establish the calculation oracle

- [x] Pin and inspect upstream loading, calculation, mutation, and export seams.
- [x] Decode the supplied PoB export without altering source bytes; record provenance,
      game/tree identity, and structural checks.
- [ ] **M1.1 Runtime — implemented and validated on Windows; hosted checks pending.**
      Embedded LuaJIT and static UTF-8 boot the pinned wrapper and load the supplied build.
      Host callbacks supply paths, time, logging, noninteractive failures and controlled
      scratch writes. Dynamic native loading is disabled; each VM stays in its worker.
      No pin change or syntax overlay was needed. Source-manifest reproducibility,
      mismatch rejection and native provenance checks pass; record hosted Windows/Linux
      results while retaining exact dependency and native source identities.
- [ ] **M1.2 Boundary — partly complete.** Fresh-process JSONL handshake/request/response,
      deadline/exit supervision, bounded I/O, retained diagnostics, parent scratch cleanup
      and CLI/XML export pass local checks. Explicit skill/scenario request selection
      remains to implement. Outputs are typed transport structures containing raw PoB
      fields, not certified game metric contracts.
- [ ] **M1.3 Import and metrics — container/preflight complete; metric semantics partial.**
      Bounded XML/share imports preserve unknown fields and source bytes; evaluator-only
      preflight prevents incomplete containers from silently calculating default builds.
      Resolve actors/groups/parts, Full DPS membership, granted/manual provenance and the
      three unresolved entries. Add a small attack/self-cast calibration case and typed
      metric availability/units; do not turn unresolved or non-finite values into zero.
- [ ] **M1.4 Baseline parity — partial.** Ordinary-frame calculation/export, completed-load
      guards and export/reimport actor comparisons pass locally. Obtain independent
      same-pin PoB reference outputs, record their production method and tolerances, and
      compare semantic state plus supported metrics. Cached export values and same-adapter
      round trips are not independent reference evidence.
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
| Runtime portability | The selected embedded LuaJIT accepts `count += 1` and the leading `#@` lines; the pinned wrapper boots on Windows without vendor changes. | Record hosted Windows/Linux results; retain version/source pins. |
| Native ABI and source identity | Static `luautf8 0.1.6` links to mlua's vendored LuaJIT. Native/Unicode tests, eight vendored-file provenance checks, source-manifest reproducibility and mismatch rejection pass locally. | Verify hosted portability and repeat provenance checks deliberately when updating pinned inputs. |
| Selected minion metric semantics | Main group is Kelari (`SummonSandDjinnPlayer`), selector 2. Fresh output names the minion action Kelari's Deception and reports raw `TotalDPS` about 60,384.684. This agrees with the cache but is not an independent reference. | Establish metric units/actor/part and scenario semantics with independent calibration. Non-finite chaos-immunity outputs are explicitly listed; define typed handling before scoring. |
| Full DPS membership | The supplied build still has no included Full DPS groups and raw player `FullDPS` is zero. | Make inclusion/count/uptime explicit; do not sum per-skill outputs or treat the zero roll-up as total build damage. |
| Unresolved and granted skills | Exactly three imported entries remain unresolved: Spectre: Powered Zealot, Navira's Well and Kelari's Deception. A live minion action with the last name does not resolve its separate imported entry. | Resolve through the adapter and reconcile manual/tree/item-granted provenance; do not omit, double-count or score unresolved entries as zero. |
| Host behavior and diagnostics | Rust handles decoding/time; host callbacks prevent prompts and redirect diagnostics. Successful startup can still log missing-node messages, which are retained in snapshots. | Review messages for coverage impact; scratch-cleanup and timeout checks already pass. |
| Legality, freshness and parity | The fresh-process ordinary-frame path, completed-load/version guards, evaluator preflight and A/B/A/round-trip checks pass locally. Full legality and independent parity remain unproven. | Calibration/reference comparison, then legal/coupled mutation and isolation coverage before reuse or fast paths. |

These are technical unknowns, not requests for the user to guess internal PoB IDs.
Ask for user input when a reproduced issue forces a product trade-off or the supplied
material cannot establish an intended build choice; present the concrete alternatives then.

## Validation evidence

### Current M1 evaluator checkpoint — local validation complete

Recorded on 2026-09-07 on Windows/x64 with Rust 1.93. The implementation is ready for
commit/push; hosted CI results are still outstanding.

| Check | Result / limit |
| --- | --- |
| `cargo fmt --all -- --check` | Passed. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed. |
| `cargo test --workspace --all-targets --locked` | **47 passed**: 13 importer, 9 preflight, 6 source, 2 runtime, 11 supervisor, 2 native and 4 CLI tests. One ignored subprocess fixture is explicitly invoked by the timeout test. |
| Fresh-worker isolation and export | A/B/A changes level 96 → 95 → 96. Export/reimport compares all finite actor metrics using `1e-8 * max(abs(expected), 1)` tolerance, alongside actor/summary checks. This is same-adapter evidence. |
| Container/evaluator separation | Empty `PathOfBuilding2` imports as a valid container but evaluation rejects it. Unsupported target versions reject before returning default/startup metrics. |
| Output safety and supervision | Output aliases reject before evaluation; bounded I/O, blocked-stdin deadline/kill/reap and scratch cleanup tests pass. |
| Sources and native provenance | Original source/decoded fixture hashes unchanged; pinned submodule clean. All eight vendored native/header file identities match provenance. Source manifest reproduces and wrong-pin rejection passes. |
| Hosted CI / independent PoB reference / joint mutation parity | Not established. Hosted checks await publication; independent calibration and all-dimension mutation coverage remain M1 work. |

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
| 2026-09-07 | M1 evaluator checkpoint (locally validated; commit/push next) | Added bounded import, evaluator preflight, embedded/native hosting, source verification, fresh-worker supervision and CLI evaluation/export. Formatting, Clippy and 47 local tests pass; hosted CI, independent parity and optimization remain outstanding. |

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
