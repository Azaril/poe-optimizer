# Implementation log and resume point

Last updated: 2026-09-07

Current phase: M1 evaluator coverage and M2 reusable core are advancing together. Configurable
scalar objective assessment, saved-result validation, a four-case attack/weapon/support
reference matrix, and native numeric modifier aggregation are implemented at this checkpoint.
All 107 local tests, formatting, Clippy with warnings denied, portable core/native WASM
compilation and the standalone objective/reassessment smoke pass. Document links, whitespace
and independent code review pass. Code is published through `86526ae`; both Windows/Linux
hosted jobs pass, including WASM compilation. Historical evidence remains below. No optimizer
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

1. Inspect `git status --short --branch` before editing. Code is published through `86526ae`
   (core/calibration/native changes in `bb08697`, CLI integration in `86526ae`). This checkpoint
   adds objective assessment, recorded-result validation, attack interaction calibration,
   native modifier aggregation and topology findings. Read its validation table below.
2. All 107 local Windows tests, formatting, Clippy, core/native WASM compilation, source/
   reference audits and standalone objective/saved-assessment checks pass. Both hosted
   Windows/Linux jobs pass for `86526ae`; next implementation work starts at item 4.
3. Keep the PoB submodule at `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4` and unmodified.
   Preserve original user fixtures and all six independently generated calibration goldens.
   The attack reference uses separate scripts; original Spark scripts/goldens remain unchanged.
   Never refresh expected values using the Rust evaluator.
4. Implement canonical candidate/domain/lock models across all six dimensions: class,
   ascendancy, passives, equipment, support gems and supporting active skills. Preserve 1..N
   required skills and 1..N exact equipped item instances, including at least two of each
   in fixtures. Use the shared evaluation and scoring boundaries; a scalar assessment is
   available now, while candidate mutation and search remain unimplemented.
5. Add tiny exhaustive joint-search fixtures and a coordinated-change trap, then controlled
   PoB mutations with requested-versus-realized state checks, legal point/resource accounting,
   lock preservation, fresh re-evaluation and export/re-import parity. The attack matrix
   demonstrates a real weapon/support ranking reversal; it does not replace six-dimension
   mutation or full legality tests. Resolve class/ascendancy ownership explicitly.
6. Use [the topology investigation](tree-topology-investigation.md) when extracting the
   candidate graph. Every actual catalog class has two surviving ordinary entrances; several
   edges are listed only in reverse. Build bidirectional adjacency, retain dangling-edge
   diagnostics and separate pinned-graph consistency from unverified game completeness.
   The missing references do not remove cross-class search from scope.
7. Continue native translation through bounded conditional/tag/scoped modifier evaluation
   and a versioned, validated data extraction boundary. Untagged BASE/INC/MORE/OVERRIDE now
   has a native implementation; unsupported metadata must remain explicit rather than be
   stripped. Capture real modifier contexts for differential validation before integration.
   Neither numerical kernels nor aggregation is a native full-build backend.
8. Add worker pools, Rayon scheduling, cancellation, caching and persistent reuse only with
   shared CPU/memory/attempt accounting and fresh-process parity. Portable WASM compilation
   remains separate from browser execution, bindings and measured performance.
9. Replace this resume point and record evidence at the next checkpoint.

No product-scope answer blocks these tasks. Exact objective metrics, thresholds, scales,
required skill/item subsets and encounter assumptions remain explicit per-run inputs.
Assessment reports constraint evidence and primary availability; it does not certify build
legality or turn diagnostic calculation output into a recommendation. All calibration cases
compare independent hosts/extractors using shared PoB calculations, not independent game models.

## Current repository and capabilities

- Branch: `main`; remote `origin` is `https://github.com/Azaril/poe-optimizer.git`
  ([repository](https://github.com/Azaril/poe-optimizer)). The user authorized publishing the
  project and supplied builds. Evaluator baseline `71c0af7` passed
  [hosted CI](https://github.com/Azaril/poe-optimizer/actions/runs/34147041293); the previous
  shared-contract checkpoint `7dd2a09` is published with complete evidence below. Current
  objective/calibration/modifier changes are published through `86526ae` with local tests/Clippy/WASM passing.
- [Workspace](../Cargo.toml): Rust 2024 / minimum 1.93; shared application contracts, PoB
  import/runtime/supervision, a static native UTF-8 library, native calculation kernels,
  and the root CLI. The [core](../crates/poe-optimizer-core/src/evaluation.rs) now separates
  `CalculationBackend` from `EvaluationEngine`. Requests/results carry build documents,
  options, capabilities, typed measurements, coverage and versioned identity without exposing
  Lua or process APIs. The current `Engine` is synchronous. Contract validation rejects
  missing/duplicate/undeclared measurements, bad units, invalid finite metadata, and changed
  backend/request identity. Backends enforce their own deadline and return typed failures.
  `EvaluationResult::validate_recorded()` is shared by the live engine and saved assessment:
  all query IDs/versions/duplicates, finite-tag values, recorded options, elapsed/context
  numbers and optional coverage numbers are checked, including unused measurements.
  Coverage schema 1 is supported. This does not authenticate source identity, reinterpret
  units using today's catalog, or certify data semantics and game legality.
- [CLI](../src/main.rs): `import`, `evaluate`, `metrics`, and `assess`. Evaluation uses report schema 2
  with a shared typed `evaluation` object; the private JSONL worker protocol is version 2.
  `--options <json>` selects one-based groups/actions and applies a named encounter's enemy
  level, boss kind and explicit five-component incoming hit. Repeatable `--metric` filters
  typed actor queries; `--raw` includes the backend diagnostic snapshot attachment.
  `evaluate --objective <json>` compiles the objective against the backend's typed catalog
  and retains its required measurements alongside an explicit metric filter. `assess`
  reads a schema-2 saved evaluation and a new objective without launching Lua or using
  today's PoB catalog. Its diagnostic assessment preserves metric schema evidence and
  recorded context/coverage. JSON/XML destinations must be new paths. Planner conversion
  is not implemented.
- [Objective assessment](objective-assessment.md): core `ObjectiveSpec` compiles into the
  backend-neutral `ScoringPolicy` interface. The current policy maximizes/minimizes one
  typed metric and evaluates a conjunction of configurable strict/inclusive constraints.
  Units, finite thresholds, positive violation scales, unique IDs and contradictory bounds
  are checked. Results expose primary value/oriented score, per-constraint observations,
  shortfalls, normalized violations and strict-boundary flags. A failed strict equality
  has zero distance but remains violated. Missing/nonfinite observations remain explicit
  and make the assessment unavailable; they are not rewarded as numeric scores.
- [Metric catalog](../crates/poe-optimizer-pob/src/metrics.rs): 15 definitions, 17 actor queries.
  Player metrics cover life/mana/energy shield, four capped resistances, PoB EHP and five
  maximum-hit values; selected hit DPS and average hit also support the selected minion.
  Units and `Finite` / `NonFinite` / `Unavailable` are explicit. `CombinedDPS` and `FullDPS`
  remain raw diagnostics, not catalog objectives. Broader action semantics, including the
  selected minion's cooldown-adjusted rate, remain diagnostic rather than rotation guarantees.
  The new Mace cases expose a mapping gap: upstream attack AverageHit is stored per hand,
  so typed selected average hit stays unavailable until hand aggregation semantics are
  defined. Typed selected hit DPS is available for these cases.
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
- [Independent calibration](calibration-reference.md): the two level-1 Spark cases retain
  19 raw numeric outputs each. Four new Mace Strike cases cross Wooden Club/Smithing Hammer
  with no support/Brutality I against the same controlled mapping enemy. Each attack golden
  records 25 top-level outputs and nine main-hand values. The preferred weapon reverses
  with the support, demonstrating that separate item/support scores cannot capture the
  interaction. Separate Windows C/Lua hosts use bundled LuaJIT `2.1.1784580905` and share
  neither production host nor extractor code. Two independent reference runs per new case
  agree exactly; [production attack parity](../tests/attack_calibration.rs) also passes
  in the 107-test combined run. This is shared-PoB host/extractor calibration,
  not independently modeled mechanics or full build legality.
- [Native engine](native-engine.md): `poe-optimizer-engine` contains the prior six numerical
  functions plus a validated untagged numeric modifier database for BASE/INC/MORE/OVERRIDE.
  Aggregation preserves flags/keyword matching, source filtering, insertion/query/parent
  ordering, rounding/truncation and override presence. Unsupported tags, value kinds,
  masks and query shapes fail explicitly. Six new differential tests execute the actual
  pinned ModStore/ModDB wrappers; conditional/scoped evaluation and data import remain
  unimplemented. Production has no external dependencies, Lua, I/O or scheduling. This
  is not a full build backend, browser app, measured speedup or replacement for PoB.
- [Tree topology](tree-topology-investigation.md): all eight actual catalog classes have
  usable ordinary entrances at six shared start locations. The 14 absent references
  remain data-coverage evidence, not proof that affected classes cannot be searched.
  Extraction must reconstruct reverse-listed edges and class-specific overrides; candidate
  validation must detect PoB normalization/pruning and mismatched ascendancy ownership.
- Candidate mutation/legality, search, Rayon pools, cancellation, search reports, recovery
  and GUI are **not implemented**. [objective.toml](../examples/objective.toml) remains
  end-state design notation; the implemented assessment input is the narrower JSON schema
  documented in [objective-assessment.md](objective-assessment.md).
- [CI](../.github/workflows/rust.yml) initializes pinned submodules and checks workspace
  formatting, Clippy and tests on Windows/Linux, plus portable core/native WASM compilation.
  Both hosted jobs pass for historical checkpoint `7dd2a09`. Hosted results for this
  objective/calibration/modifier checkpoint pass for `86526ae` in run `34152960922`.

## Delivery plan and gates

Milestone IDs are stable references for review notes. A milestone is complete only when its
acceptance evidence is recorded below. M1 and M2 may overlap after the evaluator interface
is concrete; real-build recommendations depend on both.

| Milestone | Status | Deliverable and acceptance gate |
| --- | --- | --- |
| M0: bootstrap and alignment | Complete | Rust scaffold, pinned submodule, end-state design, source/prior-art review, preserved source fixture, and this implementation record. Scaffold validation passes; this does not establish evaluator correctness. |
| M1: evaluator and fixtures | In progress; shared boundary, typed options and six independent reference scenarios validated locally; broader legal mutation coverage outstanding | Reproducible mlua/LuaJIT startup in a Rust worker, bounded import, supervised fresh-process evaluation, typed metric mappings, controlled mutations across all six dimensions, export/re-import parity, and isolation evidence. Complete the checklist below. |
| M2: reusable core and synthetic joint search | Evaluation and scalar scoring interfaces implemented; canonical candidate/lock models and synthetic joint search next | Candidate/domain/lock models, metric policies, coupled mutations and repair, Rayon/job APIs, shared budgets and events. Match exhaustive tiny domains, escape coordinated-change traps, preserve multiple locks, and verify deterministic one/many-worker results plus cancellation/dedup/accounting. |
| M3: first usable joint optimizer | Not started | Integrate all six dimensions with multicore PoB evaluation; ship preflight, evaluation/comparison, search, saved-result reranking, recovery, verified exports, and offline reports. Meet the end-to-end gates below. |
| M4: broader catalogs and upgrade workflows | Not started | Extend mechanic/equipment/skill coverage and conditional upgrade/bundle ranking with explicit inventory, cost, and comparison semantics. Retain parity and lock guarantees. |
| M5: richer objective policies | Not started | Unit-checked expressions, composite and ordered priorities, soft preferences, Pareto selection, and explicit robust aggregation. Test policy-specific selection and preserve hard constraints. |
| Desktop GUI | Deferred until CLI/report contracts stabilize | Choose frontend; Tauri is a candidate. Reuse core jobs, results and comparison models. Verify CLI/GUI parity, responsive cancellation and native-worker packaging. Does not depend on finishing every M4/M5 feature. |
| Native Rust calculation replacement | Active in parallel | Six numerical kernels and untagged numeric modifier aggregation implemented. Conditional tags, scoped evaluation, data extraction and complete pipeline slices need differential parity before exposing a native build backend. Performance and browser execution remain unmeasured. |
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
      Two self-cast Spark scenarios retain independently hosted/extracted same-pin PoB
      outputs and prior production parity. Four Mace/weapon/support references now add a
      ranking-reversal interaction; production comparison passes in the combined 107-test
      run below. Per-hand attack AverageHit remains explicitly unavailable in
      the typed catalog. Reference generation uses different host, extractor and LuaJIT
      builds with shared calculations, so this is not independent game-mechanics certification.
      Expand to minion/usage and survival cases, more equipment/support interactions, and
      actual allocated paths. Same-adapter A/B/A/export comparisons remain useful separate
      evidence; cached export values are not a reference oracle.

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

M2 now has the initial objective/scoring boundary: one user-selected registered metric to
maximize/minimize and a conjunction of typed constraints, including strict/inclusive
comparisons. Unknown metrics, invalid units, contradictory bounds, nonfinite thresholds
and unsupported policies reject. Each constraint has an explicit positive violation scale.
The current API assesses a supplied result; candidate generation, ranking selection and
search remain unimplemented. It preserves metric schema evidence and keeps scoring
independent of game metric names; M5 extends policy implementations without redesigning
candidate search. Saved assessment is a new analysis of recorded values, not a new search.

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
| Host behavior and topology | Fourteen startup messages identify absent targets in tree `0_5`; all are also absent in the four bundled older trees. Bidirectional reconstruction retains two ordinary entrances per start location and catalog ascendancy starts. Export filtering plausibly explains the dangling references without proving their meaning. | Keep all actual catalog classes in scope; validate routes, overrides, point budgets and requested-versus-realized state on the pinned graph. Preserve diagnostics and distinguish internal consistency from unverified game completeness. See [the investigation](tree-topology-investigation.md). |
| Legality, freshness and parity | Fresh-load guards and A/B/A/round trips pass at the previous checkpoint; Spark and the new Mace matrix provide independent-host reference evidence. Objective assessment evaluates typed numbers and structural replay validation checks records; neither certifies game legality or source authenticity. | Complete legal/coupled mutations across all six dimensions and failure/isolation coverage before reuse, fast paths or real-build recommendations. Preserve exact locks and reject silent pruning or changed identities. |

These are technical unknowns, not requests for the user to guess internal PoB IDs.
Ask for user input when a reproduced issue forces a product trade-off or the supplied
material cannot establish an intended build choice; present the concrete alternatives then.

## Validation evidence

### Current objective, attack interaction and native modifier checkpoint — local checks passed

Implemented on 2026-09-07, published through `86526ae` after `7dd2a09`. The combined run passed 107 tests, Clippy with
warnings denied, and portable core/native WASM compilation. Reference/source audits also
pass. The standalone objective/saved-assessment smoke and final formatting/Clippy rerun
also pass. Document checks and independent code review pass; both hosted jobs pass for `86526ae`. Historical results
below retain their original code identities.

| Check | Current evidence / remaining gate |
| --- | --- |
| Configurable core objectives | Ten new core tests cover configurable metric IDs, maximize/minimize, typed units/schema evidence, strict/inclusive constraints, contradictory bounds, shortfalls, unavailable/nonfinite primary/constraint inputs and malformed policies. Passed in the combined local run. |
| Recorded-result structure | Nine new core tests cover all measurements, versions/duplicates, finite tags, elapsed/context/options, all seven optional coverage numeric fields, coverage schema and live-engine reuse. No current catalog or Lua is required. Passed in the combined local run. |
| Objective CLI | Three new tests exercise direct evaluation assessment, fresh assessment of saved reports without a calculation process, required metrics with filtering, and rejected invalid inputs/corrupt recorded evidence. Passed in the combined local run. |
| Independent attack reference generation | Four committed Mace goldens were generated by the separate C/Lua attack host; two fresh runs per case matched 25 top-level outputs and the complete detail block exactly. Each records nine main-hand values and source/fixture/runtime/generator provenance. Original Spark artifacts remain unchanged. |
| Attack CLI regression | One new regression covers all four cases, fixture/source identity, selected skill/applied support data, 25 raw outputs, typed hit DPS and explicit unavailable attack average hit. The weapon preference reverses when adding Brutality I. Passed in the combined local run; this is not full legality or six-dimension mutation proof. |
| Native modifier differential tests | Six new tests cover the untagged BASE/INC/MORE/OVERRIDE subset against actual pinned ModStore/ModDB, flag/keyword helpers, source filtering, parent/query order, rounding/precision and unsupported input rejection. Existing five numerical-kernel tests remain. Passed in the combined local run. |
| Workspace Clippy and tests | `cargo clippy --workspace --all-targets --locked -- -D warnings` passed. `cargo test --workspace --all-targets --locked` passed: **107 tests, zero failures**, with the ignored subprocess helper explicitly invoked by its deadline test. `cargo fmt --all -- --check` and Clippy were rerun successfully after the final CLI help/comment edits. |
| Portable core/native libraries | `cargo check -p poe-optimizer-core -p poe-optimizer-engine --lib --target wasm32-unknown-unknown --locked` passed. Compilation is not browser execution, numerical browser parity or a performance result. |
| Topology investigation | Read-only source/exporter/graph inspection records actual class catalogs, both ordinary entrances per start, reverse-listed adjacency, absent references and normalization risks. No submodule modification or game-data completeness claim. Proposed fixture plan remains unimplemented. |
| Source and reference audit | Passed for all four new attack cases and both retained independent runs per case. Original fixtures and Spark artifacts remain unchanged. Source/golden identity checks pass. |
| Standalone objective and saved assessment | Passed: ignored `runs/m2-assessment-20260907-1d4afb62.json` and `runs/m2-assessment-20260907-1d4afb62-assessed.json` contain identical `ObjectiveAssessment` values from fresh calculation and saved reassessment. PoB EHP is `22562.9609905472`; fire/cold meet 75%, while lightning at 71% misses by 4 percentage points with normalized violation 0.4. Adapter fingerprint `8be35ce741554f294114ccdebd34602172f6125ff05faaf23fdc4d54e4493695`; evaluator elapsed `2591.3717` ms in this debug run. One observation is not a performance benchmark. Source pin/hash unchanged. |
| Document checks and publication | README/docs relative file links and `git diff --check` pass. Independent review found no outstanding issues in the scalar scope after recorded-result/schema-evidence fixes. Published through `86526ae`; [Windows/Linux CI run 34152960922](https://github.com/Azaril/poe-optimizer/actions/runs/34152960922) passed both jobs, including formatting, Clippy, full tests and portable core/native compilation. The following resume-document update changes documentation only. |
| Joint optimization / scaling / native whole-build backend | Not implemented or measured. New assessment and coupled reference cases do not establish search quality, throughput, full native parity or browser viability. |

The [objective notes](objective-assessment.md), [attack reference notes](calibration-reference.md),
[native coverage](native-engine.md), and [topology investigation](tree-topology-investigation.md)
record current contracts and limits. Diagnostic saved assessment retains backend/context/coverage
and exact assessed measurement schema versions. Its validation cannot authenticate an edited
report, and no current PoB catalog is substituted for recorded metadata.

### Historical contracts, calibration and native checkpoint — published and validated

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
| 2026-09-07 | `bb08697` + `86526ae` | Added configurable scalar assessment and saved-report validation, four independent Mace interaction goldens, native untagged numeric modifier aggregation and a source-level tree topology investigation. All 107 local tests, formatting, Clippy, core/native WASM compilation and standalone objective/reassessment smoke pass; source/golden audit passes. Document checks and independent code review pass; published through `86526ae`, with both hosted Windows/Linux jobs passing in run `34152960922`. Canonical all-dimension candidates, locks and joint search remain next. |

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
