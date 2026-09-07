# Implementation log and resume point

Last updated: 2026-09-07

Current phase: M0 complete; M1 evaluator spike is next. No evaluator or optimizer is implemented.

This is the living record of delivery order, implemented behavior, validation, unresolved
work, and the next session's starting point. [Design](design.md) defines the intended system
and acceptance criteria; [execution and interfaces](execution-and-interfaces.md) defines
runtime and application contracts. The [PoB investigation](pob-integration.md) and
[prior-art review](prior-art-and-product-review.md) retain dated evidence and rationale.
Update this document at progress checkpoints, rather than adding implementation status to
the design documents.

## Resume here

1. Read this section and the current M1 checklist below. Inspect `git status --short --branch`
   before editing; preserve any user changes.
2. Confirm the PoB submodule is at
   `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4` using `git submodule status`.
   Initialize the recorded revision if needed; do not update to upstream HEAD as routine setup.
3. Start **M1.1: provision and parse/boot the external LuaJIT runtime**. Verify the
   `Main.lua` syntax concern before building the full protocol or tuning worker reuse.
   Keep runtime setup, generated settings, and any explicit compatibility overlay outside
   the source submodule. Record exact setup commands and runtime/module identities here.
4. Once startup works, establish fresh-process evaluation of a small calibration build and
   the supplied minion fixture. Determine actual metric/skill coverage before making
   optimization recommendations.
5. At the next checkpoint, replace this resume point with the next concrete action, record
   evidence and failures, and update the milestone checklists. Do not mark M1 complete
   on the strength of a successful fixture decode or Rust scaffold build.

No further product-scope answer is required before this spike. Runtime compatibility and
fixture ambiguities are engineering investigations. Exact objectives, thresholds, required
skill/item subsets, and encounter assumptions remain explicit per-run inputs.

## Current repository and capabilities

- Branch: `main`. GitHub repository:
  [Azaril/poe-optimizer](https://github.com/Azaril/poe-optimizer).
  Remote `origin`: `https://github.com/Azaril/poe-optimizer.git`.
- Publication permission: on 2026-09-07, the user explicitly authorized pushing this project,
  including the supplied source exports and decoded fixture, to this public repository.
  The repository was created specifically for this project; no publishing question is pending.
- Last implementation baseline: `72bf6a7` (design review and decoded fixture).
  Documentation checkpoint `8eb2e23` introduced the living implementation record.
- [Rust package](../Cargo.toml): edition 2024, declared minimum Rust 1.93, stable toolchain,
  no library dependencies. [CLI scaffold](../src/main.rs) supports help/version only.
- The PoB submodule is pinned and unmodified. It is source-inspected, not runtime-validated.
- Original exports and a decoded XML fixture are tracked with hashes and structural metadata.
  No production import parser exists yet.
- Core/adapter/report workspace crates, evaluator protocol, search, Rayon scheduling,
  objective parser, reports, recovery, and GUI are **not implemented**.
  [objective.toml](../examples/objective.toml) is design notation, not accepted CLI input.
- [CI](../.github/workflows/rust.yml) is configured for Rust formatting, Clippy, and tests on
  Windows and Linux. It does not install or test Lua/PoB.

## Delivery plan and gates

Milestone IDs are stable references for review notes. A milestone is complete only when its
acceptance evidence is recorded below. M1 and M2 may overlap after the evaluator interface
is concrete; real-build recommendations depend on both.

| Milestone | Status | Deliverable and acceptance gate |
| --- | --- | --- |
| M0: bootstrap and alignment | Complete | Rust scaffold, pinned submodule, end-state design, source/prior-art review, preserved source fixture, and this implementation record. Scaffold validation passes; this does not establish evaluator correctness. |
| M1: evaluator and fixtures | Next; source inspection and fixture decoding only are complete | Reproducible LuaJIT startup, bounded import, supervised fresh-process evaluation, typed metric mappings, controlled mutations across all six dimensions, export/re-import parity, and isolation evidence. Complete the checklist below. |
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
- [ ] **M1.1 Runtime:** provision a pinned LuaJIT executable and compatible `lua-utf8`;
      record versions, hashes, architecture/ABI, module paths and setup commands. Parse
      `Main.lua` and boot `HeadlessWrapper.lua`. Reproduce or dismiss the syntax concern.
      If needed, compare a known passing pin or an explicit minimal compatibility overlay;
      record the reason, diff/hash and parity evidence. Never conceal a vendor edit.
- [ ] **M1.2 Boundary:** create the first functional core/PoB/CLI workspace packages as
      needed; keep the report package until it has functionality. Implement versioned
      JSON Lines handshake/request IDs, raw XML input, selected skill/scenario, typed
      outputs, structured errors, deadline/exit supervision, bounded output and XML export.
      Use one request per fresh process first. Keep diagnostics off protocol stdout,
      prevent interactive prompts, and control writable paths/environment.
- [ ] **M1.3 Import and metrics:** implement bounded URL-safe base64/zlib and raw XML import
      with DTD/entities disabled, complete-stream validation and unknown-field preservation.
      Add a minimal self-cast or attack calibration fixture alongside the complex minion
      fixture. Resolve actors/groups/parts, Full DPS inclusion, granted/manual provenance,
      and missing IDs. Report unavailable or unvalidated coverage explicitly.
- [ ] **M1.4 Baseline parity:** evaluate through the ordinary PoB frame path and compare
      with independently obtained outputs from the same pinned PoB/runtime. Record how
      reference values were produced and numeric tolerances. Cached import values are
      not reference evidence. Save/re-import XML and compare semantic build state and
      supported metrics. Missing required metrics, non-finite values, stale outputs and
      invalid skill selection must fail visibly.
- [ ] **M1.5 Mutation parity:** exercise legal passive, item, support, supporting-skill,
      class and ascendancy changes, first separately and then in coordinated candidates.
      Preserve required skills/items and all declared locks. Recompute candidate-derived
      effects while holding external assumptions fixed. Compare with fresh materialization
      and reload; verify exported state, point accounting, resources and effect removal.
      Include fixtures with at least two required skills and two equipped-item locks.
- [ ] **M1.6 Isolation and failures:** run A/A, A/B/A, permuted request order, cold/warm
      comparisons, malformed input, timeout and worker-failure cases. Only enable persistent
      reset/reload workers after matching the fresh-process reference. Compare fixed
      candidates across worker counts within declared tolerances.
- [ ] **M1.7 Measurements:** record cold startup, full-evaluation and export/reload time,
      peak memory and error rate on identified hardware/runtime. Benchmark warmed/reset
      workers and marginal overrides only where correctness is proven, with EHP enabled
      whenever needed. Use results to choose worker limits; do not invent throughput targets.

The planner JSON export is preserved as auxiliary input/provenance. Raw PoB XML and PoB
share-code decoding are the initial runnable import paths; planner conversion follows only
when its format and missing semantics can be mapped faithfully.

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
| Pinned runtime compatibility | Source `Main.lua:339` contains `count += 1`; headless loading uses `loadfile`. No interpreter reproduction yet. | M1.1 parse/boot; record exact failure or successful compatibility mechanism. |
| Runtime availability and native ABI | Windows DLLs are in upstream; no Lua/LuaJIT executable was found on PATH during source investigation. Docker executable was found, but engine availability was not tested. | Select and record reproducible runtime/module setup; verify architecture and actual startup. |
| Selected minion metric semantics | Main group is Kelari (`SummonSandDjinnPlayer`), selected minion skill 2; all Full DPS flags are literal `nil`, cached player FullDPS is zero. | Resolve actor/part and inclusion rules in PoB. Do not substitute cached minion damage or sum skill outputs blindly. |
| Unresolved and granted skills | Spectre: Powered Zealot, Navira's Well and Kelari's Deception lack stable IDs; manual/tree/item-granted groups need reconciliation. | Resolve through adapter and record diagnostics; do not silently omit, double-count or score unresolved entries as zero. |
| Placeholder host functions and prompts | Headless compression/time functions are stubs; startup/frame errors can prompt for input. | Use Rust decoding/timing, raw XML, controlled callbacks and supervised failure handling. |
| Legality and state freshness | Source has caches, marginal overrides and partial calculation paths. No evaluation or export parity evidence yet. | Full-candidate checks, ordinary-frame reference, coupled mutation and isolation tests before reuse/fast paths. |

These are technical unknowns, not requests for the user to guess internal PoB IDs.
Ask for user input when a reproduced issue forces a product trade-off or the supplied
material cannot establish an intended build choice; present the concrete alternatives then.

## Validation evidence

Recorded on 2026-09-07 for baseline `72bf6a7` and its scaffold ancestors:

| Check | Result / limit |
| --- | --- |
| `cargo fmt --all -- --check` | Passed. |
| `cargo clippy --all-targets --locked -- -D warnings` | Passed on Windows with Rust 1.93.0. |
| `cargo test --all-targets --locked` | Passed, **zero tests**; verifies only the starter target builds. |
| CLI help/version/invalid argument | Checked; invalid arguments exit with code 2. |
| Fixture decoding and structure | Source/output hashes checked; bounded complete zlib decode and XML parsing; 130 tree IDs checked. This was an investigation, not an implemented importer test suite. |
| Submodule and documentation | Recorded pin clean; local document links and `git diff --check` passed at the prior checkpoint. |
| Lua evaluation, mutation/export parity, search/scaling | Not run; no implementation exists. |
| Hosted CI | Workflow configured; no hosted run result recorded at this checkpoint. |

The living-document checkpoint changed documentation only. Local file/heading links and
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
| 2026-09-07 | Publication authorization (this change) | Recorded explicit user authorization to push the project and supplied fixtures to GitHub; cleared the pending publishing question. |

## Decisions still deferred

No answer is needed before M1. Choose the project's distribution license before a public
release; confirm the desktop framework and packaging before GUI work; choose acquisition
data sources before trade/upgrade ingestion. Benchmark-specific metrics and usage profiles
must be documented when those fixtures are made runnable, without turning their choices
into mandatory goals for all users.
