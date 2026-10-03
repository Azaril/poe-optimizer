# Implementation plan and resume point

Updated: 2026-10-03 (EDT).

This is the active delivery plan. The [design](domain-architecture.md) defines
the end state; the [execution overview](data-and-evaluation-overview.md) explains
what actually runs. Older checkpoint reports are preserved in
[implementation history](implementation-history.md), not current instructions.

## Current state

**Five supplied builds: 0/5 complete native evaluations.** All five import and
run in pinned PoB. Owned native components work, but unresolved inputs and
mechanics still prevent complete native requests. Component parity, catalog size
and deleted code are not substitutes for this gate.

The owner confirmed that **SQLite, DuckDB and an ORM are outside the plan**.
Use generated owned artifacts and immutable loaded Rust indexes. New features
belong in one native path:

| Subsystem | What belongs here |
| --- | --- |
| Owned Core | Source-independent build, occurrence, scenario and request contracts |
| Owned Data | Injected definitions, tables, typed rules, identities and compilation |
| Owned Engine | General native preparation, effects and metrics; reusable worker scratch |
| Owned Import | External build decoding, source interpretation and checked normalization |
| Offline acquisition | Convert pinned source facts to reviewed owned data artifacts |
| Optional PoB | `mlua`/LuaJIT reference execution and data acquisition |
| Legacy Import/Data/Engine | Only named remaining acquisition, inspection or numerical-reference consumers; retirement work |

`evaluate-owned` is the native CLI entry point. `evaluate` and `metrics` are
optional PoB reference commands behind `--features pob`. Native evaluation has
no PoB subprocess or Lua state. Some legacy Rust modules still compile through
Import's dependencies; full legacy distribution isolation is unfinished.

The obsolete native CLI, NativeBackend crate, class/UI experiments and orphaned
profile-template coordinator are removed. Keep useful independent source and
numerical tests. Delete unused paths together with their exclusive scaffolding;
an exported API or its self-tests alone do not justify retention. The
[retirement inventory](legacy-retirement.md) names the remaining consumers.

## Latest checkpoint: Ice Nova physical inputs with deferred usage

Subsystems changed: owned Import, reviewed normalization data, optional PoB
source observation and Rust tests. The additive physical V3 policy distinguishes
intrinsic Gem assignments from reference-action settings and explicitly deferred
usage. It reuses typed value recipes and the existing action correspondence;
Core, Engine and the owned rule language need no new model or interpreter.
Legacy V1/V2 policies retain their contracts. No legacy consumer was retired in
this checkpoint.

The four Ice Nova occurrences in Original05 now have complete intrinsic input
lists. Their already-known values, local IDs, allocation watermark, actual
preset preferences and Pending usage obligations are preserved. Exact source
links account for the deferred fields. Completion requires a real same-preset
Pending destination and exhaustive reference-child coverage; duplicate/aliased
preset IDs, unknown fields, malformed values and unaccounted maps cannot close
an inventory. A shared container check retains the older flat-row census.

The complete source witness passes all 23 cases in both JIT modes, retaining
fresh load and two requested rebuilds independently. Its equal reports record
count precedence, duplicate matching, FullDPS, independent stat sets, disabled
sources and archived isolation. These observations do not select a canonical
parity lifecycle or claim universal inertness of usage fields.

Publication checks every field and provenance link in all five original drafts.
Only the four intended intrinsic completions and exact Pending-usage links change.
All 110 query rows remain byte-identical; eighteen package files rebuild exactly.
Selected obligations are now **119 / 116 / 108 / 121 / 18**, with Original05 down
from 19. All other selected obligations are identical. Eight mutation controls
compare both policies on the same edited XML, preserving pre-existing conservative
usage behavior; they do not broaden older parsers to admit new shapes.

Validation passes: **498 Import tests**, both CLI tests (including the real
publication), the two-mode source witness, strict workspace/native Clippy, both
WebAssembly configurations, compiled boundaries, native dependency closure and
all-package formatting. No full workspace runtime test run or hosted-CI pass is
claimed. The mechanics registry remains Partial, there is no evaluation bundle,
and complete native originals remain **0/5**.

Evidence: `runs/owned-ice-nova-inventory-02/validation.json`,
`runs/owned-ice-nova-inventory-import-02.log` (191 library and six compact
transition tests), `runs/owned-ice-nova-inventory-import-04.log` (301 integration
tests), `runs/owned-ice-nova-inventory-cli-02.log`,
`runs/owned-ice-nova-inventory-final-checks-01.json`, and the
[source-action and disposition contract](owned-source-actions.md).
The source reports are under `runs/owned-ice-nova-occurrence-source-01/`.
`runs/owned-ice-nova-inventory-checkpoint-01.json` records the validation logs;
`runs/owned-ice-nova-inventory-02/selected-05-report.json` contains the current
eighteen exact selected-request obligations for the next session.

## Checked baseline and original-build results

Use `runs/owned-ice-nova-inventory-02/package` as the current integration baseline.
Its predecessor is `runs/owned-ice-nova-actions-04/package`. Publication requires
the exact predecessor and authenticated passing Ice Nova occurrence reports in
both JIT modes. Checked-in authoring is
`data/owned/poe2/3887ae68/ice-nova-inventory/`.

| Identity | Current value |
| --- | --- |
| Source revision | `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4` |
| Release input | `4b1823e1ac6fa26f9115764ebb1b77b2066aec0bf778e8e19919fb65713e07b0` |
| Registry | `b4708f62fbaa607caa3815b5da9925caa1876371df4056d182542d17252ec900` |
| Definitions content | `92ad20da2a483a277af6a69e9aa2e10d30d5ca4a5bbebb2fb123eac27d3de707` |
| Normalization | `8e9cfb20e10ca0b823d82c6f7c7e6b8ac099c048320ff0483ac63248dcc99bf9` |
| Tree policy | `191f5cba0bc1f7fe3e79bd0bb5b8d7a9aad9863960aea105382cc74be0717af4` |
| Schema / operations | V5 / `owned-domain-operations-v17` |

The eighteen package files total 60,430,352 bytes and contain 67 provenance rows.
All files rebuild byte-identically. The Partial effect-application registry and
absence of an evaluation bundle are preserved. Real final-input producers and
complete mechanics remain unfinished.

| Original | Unchanged saved selection | Selected unresolved issues | Native evaluation |
| --- | --- | ---: | --- |
| 01 | Kelari / Sand Djinn, Kelari's Deception | 119 | Not run: Pending |
| 02 | Twister, skill set 6 | 116 | Not run: Pending |
| 03 | Whirling Assault, average-damage mode | 108 | Not run: Pending |
| 04 | Crossbow Shot | 121 | Not run: Pending |
| 05 | Skeletal Sniper, Basic Attack, skill set 4 | 18 | Not run: Pending |

Crossbow's saved reference selection has no hit-damage output; preserve its
availability result rather than choosing a different skill. Original source
bytes, saved selections, all 110 queries and every unrelated selected obligation
remain unchanged. Local `runs/` files are reproducible evidence, not distributed
game data.

## Next executable work

1. **Resolve the closest real build's input contracts.** Original05's eighteen
   selected obligations are four Gem parameter inventories, six support targets,
   two Direct parameter inventories, and one each for scenario usage, preset
   usage, skill membership, support-origin discovery, configuration roles and
   external assumptions. Configuration roles still link 255 source rows; typed
   numeric controls alone do not close this inventory.
   **Next: Sniper's own saved actor/action disposition.** Its scalar inputs,
   typed count and owned Basic Attack provider path already exist. Extend the
   bounded Import correspondence to that explicitly declared Actor child Action;
   preserve the current player-primary variant. Reuse owned actor/action IDs and
   grants rather than adding a Core/Engine model or hidden reference query.
   The five physical occurrences are source ordinals 193, 211 (selected), 254,
   310 and 375 in Original05. They belong to presets 3, 4, 5, 6 and 1; the first
   three have raw level 20, the last two level 19. Authenticate all five, including
   archived actor/action selection, rather than borrowing selected-only evidence.
   The current original query path uses grants 0017/0020/3093, Actor3091 and
   Basic Attack output0022 with Part0007/Mode0008/StatSet0009. These are injected
   artifact IDs, never production-code constants.
   Source selectors `skillMinion`, `skillMinionCalcs`, `skillMinionSkill` and
   `skillMinionSkillCalcs` need finite, context-aware correspondence. CALCS actor
   lookup depends on the selected summon; do not treat its saved field as an
   unconditional independent gameplay choice. Initially admit the witnessed
   Basic Attack alternative; unknown names, unsupported choices/maps, Gas stat
   sets and missing Commands stay explicit. Use the existing minion occurrence
   and reservation witnesses and add any missing archived-source controls.
   Target `18 -> 17` only after unchanged-build preservation and source-bound
   tests pass. This is a planned gate, not an achieved result.
   Ice Nova's intrinsic inventory is now complete. Its final inputs and
   numerical tables remain separate work: authenticate constructed per-level
   stats and additive duplicate aggregation before publishing damage data.
   **Then resolve skeletal and Direct source dispositions.** Sniper count and its
   ordinary reservation component are implemented, but minion/action selection,
   unresolved Command additions, generated descendants and six Djinn support
   destinations remain. Manual Djinn raw storage is implemented; complete input
   and usage inventories are not. Their catalog non-container proof remains valid.
   Source authority is [minion/spell evidence](owned-minion-spell-input-evidence.md)
   and the [reservation contract](owned-summon-reservation.md).
   Reservation also needs build-driven parent Action contexts: Original05's saved
   metric queries request child actions only. Discover necessary contexts from
   declared build mechanics; do not insert hidden reference queries or make
   required costs depend on the metrics a user asks to display. Keep this gap
   explicit while integrating real final-level and modifier producers.
   Rerun all five originals at each boundary, preserving Twister and Sniper as
   contrasting integration cases. Never relax coverage to create a successful
   request.
2. **Integrate readiness with source-backed final-input assembly.** The generic
   V16 contract and public support-plan proof now exist. With the real release
   now on V17, establish exact physical source-gem/effect membership and apply
   supported properties once per intended source. Preserve the fractional
   ordering witness `12 + 0.25 + 0.75 -> 13` through final validation. Use existing
   declared-child projection authority; the component proof does not authorize
   arbitrary parent reads or a self-parameter writer. Keep final action/query
   gates, ancestor requirements and incomplete contributor coverage intact.
   See the [accepted contract](owned-preparation-readiness-proposal.md). This is
   integration work alongside the measured input blockers, not another general
   evaluator or a replacement build model.
   The read-only `runs/owned-readiness-real-integration-audit-01.md` identifies
   the next source witness: exact physical-effect/support membership, count and
   source-property cache behavior. Minion child actions have no physical source
   Gem; do not infer one from ancestry. Existing external global-minion inputs
   also retain unresolved Amulet bonus-copy/routing/contributor coverage. Resolve
   those data/rule producers rather than hardcoding the observed final level22.
3. **Continue bounded retirement in parallel where files do not overlap.** Audit
   the remaining legacy Import/Data/Engine closure. Separate the shared skill
   identity and Lua-number formatting helpers used by owned conversion, then
   remove exclusive profile preparation. Preserve named acquisition consumers
   and independent numerical kernels/oracles. Do not create compatibility
   facades for APIs whose consumers have already retired.
4. **Connect final supported inputs and numerical consumers.** Parent quality,
   Gigantic, physical-range damage and remaining offence/defence depend on the
   preceding contracts. Finalize and evaluate the exact original request, retain
   every query and compare fresh reference results before claiming a working build.

Enemy distance is a distinct future input: its count-style zero fallback differs
from the raw override lane. More numeric controls alone cannot close the current
configuration or usage inventories. Do not add unrelated catalog families while
an identified selected-request blocker has a clear fix.

The usage implementation reuses `UsagePolicySelection`/`UsagePolicyDraft`, shared
validators and `RuleOrigin::Usage`. Preserve this single composition/execution
path when extending Import projection. Bind each actual source row to its newly
allocated SkillUse and containing preset, with exact effect correspondence for
generated targets. Policies must validate typed slots, source/domain guards and
work limits before traversal. Omission must preserve prior normalization bytes;
known preferences may coexist with Pending inventories. No inventory closure
follows from persistence alone.

The earlier source audit in `runs/owned-usage-import-next-audit.md` identifies the
Original05 rows. New complete-source witnesses now cover the four skeletal
families' occurrence/action/count inputs and Ice Nova's actual constructed stat
sets, in both JIT modes. See `runs/owned-minion-occurrence-inputs-01/` and
`runs/owned-spell-stat-set-source-01/`. Minion count has a real reservation
consumer; Full DPS uses separate aggregation rules. The minions' global switches
are inert in this source domain, so do not repurpose Offering's Boolean policy
as minion activation. Ice Nova has two constructed stat sets, not three inferred
from aliases. Ice action selection/physical disposition and Sniper count/reservation now have
native consumers or checked projection. Other typed consumers remain unfinished; none of this
evidence closes scenario usage by itself.

### Accepted and pending owner decisions

The owner accepted these contracts:

- **Accepted:** [skill-preset usage composition](owned-skill-usage-proposal.md), typed
  preferences on the supplying skill preset composed with exact scenario overrides.
  Composition, native execution and initial Boolean source projection are
  implemented and tested. Full inventory proofs still do not follow from them.
- **Accepted:** [preparation versus execution readiness](owned-preparation-readiness-proposal.md),
  one occurrence topology/effect graph with explicit phase dependencies. Its
  versioned declarations, compiler gates and positive public support-plan proof
  are implemented and the component, compatibility and portable checks passed.

- **Accepted:** [Direct SkillUse inputs](owned-skill-occurrence-input-proposal.md),
  shared typed slots with explicit authored/provider authority. Complete/draft
  persistence, binding and native reads pass component tests. Reviewed manual
  Djinn raw inputs now import into the V5/V17 release; full final-input and
  topology coverage remain incomplete.

**Pending owner input:** future canonical PoB parity lifecycle. The Frost witness
proves a cold MAIN/CALCS difference and stable requested-rebuild results. The open
question recommends comparing two matching normal rebuilds while retaining cold
diagnostics; matching first-load behavior is the alternative. Do not change the
reference backend's authority or encode source cache state in native rules until
this decision is resolved. Input storage and independent count work can continue.

The [socket configuration](owned-socket-configurations.md) proposal remains
unaccepted. Proceed with the accepted contracts and independent cleanup.

## Delivery plan and gates

The authoritative architecture phases are [D0-D6](architecture-migration.md#phases-and-exit-gates).
Do not reuse the older archived D1-D5 profile milestones as current instructions.

| Phase | Current state and next gate |
| --- | --- |
| D0: boundaries and retirement | Owned architecture accepted; delete obsolete paths continuously, retaining named useful references |
| D1: semantic input | Owned model/import exists; unresolved original-build input contracts remain |
| D2: offline data/compiler | Owned packages and typed programs work; mechanic coverage/conversion is incomplete |
| D3: native evaluation | Component execution works; first complete Twister/Sniper requests, then all five originals |
| D4: search integration | Generic search/objectives exist; connect owned candidates and evaluator across all six dimensions |
| D5: retirement and breadth | Active alongside D1-D4; finish legacy isolation and independent whole-build holdouts |
| D6: performance and applications | Measure real native workloads, then shared reports/events, UI and web delivery |

Stable product milestones remain: M0 bootstrap complete; M1 evaluator/fixtures
in progress; M2 generic search contracts partly implemented; M3 first usable
joint optimizer unfinished; M4 broader catalogs/upgrades and M5 richer objective
policies follow. Retired Spark/Mace profile benchmarks do not satisfy current
native performance or optimizer gates.

The first usable optimizer must jointly search classes, ascendancies, passives,
equipment, support gems and supporting skills within explicit finite catalogs.
Preserve 1..N required skills and exact item locks, configurable objectives and
constraints, bossing/mapping scenarios and configurable 5-30 minute thorough
runs. Test coordinated changes, tiny exhaustive domains, deterministic parallel
execution, cancellation, recovery, deduplication and verified exports. CLI/core
contracts come before the GUI; Tauri remains a candidate. PoE1 is a later distinct
versioned adapter.

M4 must rank conditional upgrades and bundles with explicit inventory, cost and
baseline semantics. M5 adds unit-checked derived/composite/ordered objectives,
soft preferences, Pareto selection and explicit robust aggregation while
preserving hard constraints. The GUI must reproduce CLI results through the
same libraries, support responsive cancellation and package the native evaluator
reproducibly; optional reference tooling stays separately installable.

### M1 checklist: native calculation and optional reference parity

- [x] Pin original PoB, preserve caller sources and retain optional reference hosting.
- [ ] Complete the owned request/scenario boundary across all five originals.
- [ ] Produce final metrics with explicit units, availability and contributor coverage.
- [ ] Match complete native originals and independent component references.
- [ ] Match legal mutations and interacting changes in every search dimension,
  including export/reimport and multiple exact locks.
- [ ] Prove fresh/reused/parallel isolation, bounded failures and cancellation.
- [ ] Measure real native preparation, candidate evaluation and memory separately
  from optional reference-worker costs.
- [ ] Complete versioned data conversion, update/reconciliation and broader mechanics.

### M2 and M3 acceptance details

Objectives are generic registered metrics with explicit units and coverage;
unsupported policies, contradictory constraints and nonfinite thresholds reject.
Richer policy implementations must not require redesigning candidate search.
M3 still requires all of the following through the owned evaluator:

- One joint run across all six dimensions, with legality/repair and every requested lock.
- Complete-candidate verification independent of optimization caches, with reserved
  verification budget and exact realized export/reimport comparisons.
- Shared CPU/memory limits, bounded queues, in-flight deduplication, cancellation,
  bounded retries and consistent attempt accounting.
- Preflight assumptions/coverage, baseline comparisons, grouped changes, constraint
  shortfalls, distinct alternatives, JSON artifacts and offline HTML reports.
- Atomic recovery with actual verification state; recovered seeds are freshly
  checked. A warm start gets a new budget; exact search-state continuation is separate.
- Quality against random, greedy and alternating-domain baselines across several
  seeds and equal budgets. Measure scaling and 5/15/30-minute end-to-end quality
  on recorded hardware for bossing and documented mapping proxies.

Saved-result reranking is analysis of recorded values, not a new search. Legal
infeasible exploration remains distinct from verified feasible incumbents.
The M2 synthetic suite must include an exhaustively checkable domain where every
single-change improvement path stalls but a coordinated change wins.

## Fixture ledger and technical unknowns

The [fixture ledger](../tests/fixtures/builds/README.md) preserves source identities;
the [five-build inventory](breadth-validation.md) preserves the wider working set.
Original and independently hosted numerical fixtures remain useful after profile
code retirement. Preserve selected minion/action identity, Full DPS membership,
count/usage assumptions, grants, separate point pools and nonfinite availability.
Do not silently repair unknown skills or interpret labels/tree versions as verified
game-patch identity. Ask for product direction when evidence leaves a real choice;
source-internal IDs and mechanics audits are implementation work.

## Breadth of validation and data-driven build admission — next phase

[Breadth validation](breadth-validation.md) and the
[mechanism inventory](breadth-mechanism-inventory.md) define the corpus.
All production inputs are caller supplied and data driven; fixture names must
never select calculation behavior.

- B1 caller-configured intake and provenance: implemented; preserve exact originals.
- B2 coverage inventory: partial; report mechanism gaps separately from first errors.
- B3 general input seam: owned model accepted/implemented in part; finish real inputs.
- B4 independent corpus: add held-out attacks, spells, minions, DoT, conversion,
  triggers, recovery/reservation and supporting-skill interactions with explicit scenarios.
- B5 identity/legality/interactions: preserve export/reimport, view changes, multiple
  locks and separate ordinary/ascendancy roots and point pools. Source-missing nodes
  are distinct from disconnected ascendancy trees and exceptional allocations.
- B6 usefulness/performance: measure native preparation, parallel evaluation and
  complete search on admitted real builds. Unsupported cases remain visible.

The five development originals are not holdouts. Never simplify them to manufacture
parity, regenerate expected outputs from native results or count unknown outcomes
as matching numbers. Store perturbations separately.

## Execution-model investigation: A1-A4

The [investigation brief](rule-execution-model-investigation.md) remains a follow-up
to the owner's concern about loader/parser/interpreter cost. The later owned-domain
decision controls current work; this investigation does not reopen PoB UI emulation.
A1 inventories current consumers and maintenance/runtime costs; A2 compares bounded
DSL, declarative, injected-native, interpreter and offline-lowering approaches;
A3 discusses measured results and a proposed decision with the owner; A4 implements
the agreed simplification and removes superseded paths. Compare the same real
mechanics, cold preparation, changing candidates, steady evaluation and update effort.
Keep the native/WASM path and independent parity. Unneeded experiments can be
deleted now; total retirement does not require completing every prototype.

## Definition storage and UI search

Storage decision complete: generated owned packages plus loaded Rust indexes;
no database/ORM milestone. The [storage assessment](definition-storage.md) is
historical context. At the discovery/UI phase, add a bounded query API over the
selected snapshot with stable IDs, labels, filters, coverage and a derived
prefix/substring index. Measure latency/memory before adding specialized search.

## Seeded jewel opportunity: J1-J5 follow-up

The [design opportunity](seeded-jewel-search.md) and
[feasibility study](seeded-jewel-feasibility.md) remain opt-in future work.
J1 must establish game/version applicability, compatible seed data, provenance
and costs; the pinned PoE2 seed branches and available PoE1 data are not assumed
interchangeable. J2 adds an injected provider with radius/overlap/removal parity;
J3 adds bounded parallel seed/socket/path search with explicit budgets and caches;
J4 adds reproducible item/tree reporting and trade handoff; J5 validates independent
families and compares search quality/cost under equal budgets. These gates do not
displace the five-build MVP. No native seed provider/search is implemented.

## Tooling and checkpoint workflow

New maintained tools and tests use Rust. Existing Python exporter/intake tests
remain useful; the owner explicitly deferred converting them. Their eventual
coherent replacement is [T1](architecture-migration.md#t1-rust-tooling-and-test-consolidation).

At session start, read this resume point and inspect Git state. At each coherent
checkpoint:

1. Name the subsystem changed and the actual selected-build blocker addressed.
2. Rerun unchanged saved selections for all five originals; evaluate when admitted.
3. Record exact data/source identities, commands, results and retained failures.
   Distinguish component evidence from whole-build and hosted-CI results.
4. Update this compact current state and next action. Append detailed completed
   checkpoints to history; do not accumulate competing current plans here.
5. Update design only for changed architecture/contracts, discuss significant
   direction changes, review the diff and publish a coherent tested checkpoint.

Keep Cargo invocations serialized in the shared workspace; independent tests
within a target can use bounded parallelism unless they share mutable fixtures.
For heavy legacy numerical oracles, use CI's optimized test profile while retaining
debug assertions and overflow checks; snapshot reconstruction is very slow in an
unoptimized test build. Format per package on Windows to avoid command-length
limits. Do not edit
`crates/poe-optimizer-engine/src/owned_allocations.rs` or its tests during this work.
The owner authorizes commits/pushes to main and CI-log inspection.

Deferred product decisions: distribution license before a public release,
frontend/packaging before GUI delivery, and acquisition sources before live
trade ingestion. They do not block current native build support or cleanup.
