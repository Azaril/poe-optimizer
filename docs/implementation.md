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

## Latest checkpoint: native Djinn support preparation

Subsystems changed: reviewed owned data, Rust component/CLI validation and the
optional PoB source witness. Ten real supports now have preparation data for the
existing native admission engine. This includes Original01's six additional
support definitions, rather than restricting the proof to Original05's four.
The predicates include conjunctions and exclusions; Frost Nexus adds a prepared
type. No new production Core/Engine/Import API or interpreter was added.

The [preparation packet](../data/owned/poe2/3887ae68/djinn-support-preparation/README.md)
is a standalone existing Data artifact bound to the exact integration release.
It adds no registry IDs, release files, complete declarations or evaluator bundle.
Embedding preparation in a release still requires the complete existing evaluator
artifact set; empty stages or receiving artifacts would not meet that requirement.
Its identity is `f3afe031c10d9e7a6927a4b06db3665e9e710f92b06f35a5bde084fb5e39af5b`.

All four native tests pass. They replay **1,296 exact source contexts**, use native
parent preparation for child admission, preserve exact candidate origins and
ordering, and check disabled origins, duplicate missing inputs, unknown facts,
unreviewed definitions, work-budget failure/recovery and parallel reuse. The
expected acceptance and final types come from authenticated original execution,
separately from the source definitions used to author runtime predicates.
This is component evidence with source-observed inputs, not a complete native
build binder or numeric support delivery.

Both data tests pass, including canonical codec roundtrip, stale binding rejection
and fresh normalization of all five unchanged originals. The eighteen release
files, all 110 queries and selected unresolved counts **113/116/108/121/11** are
unchanged. **0/5 native builds are complete.** The six Sniper support-target issues
remain open because their complete owners, contributors and input producers have
not yet been established. Complete-build gates were not relaxed.

The expanded source witness passes twelve cases, three lifecycle stages and both
JIT modes. All ten definitions appear in actual predicate calls, and six
independent disable controls preserve the other support occurrences. Reports
contain identical bytes across JIT modes. See [Djinn evidence](owned-djinn-provider-evidence.md#original-djinn-support-preparation-and-admission).
Passing logs are `runs/owned-djinn-support-preparation-source-03.log`,
`runs/owned-djinn-support-preparation-data-02.log` and
`runs/owned-djinn-support-admission-native-01.log`. Data attempt `-01` caught an
incorrect helper artifact count: the iterator includes the release receipt as
its eighteenth file. The correction checks its canonical bytes separately from
the seventeen artifacts listed in that receipt.

All fifteen strict checks pass: workspace/native Clippy, both WebAssembly library
configurations, compiled boundaries, native dependency closure and all-package
formatting. The default executable has no PoB/Lua dependency. Receipts are
`runs/owned-djinn-support-preparation-01/validation.json`,
`runs/owned-djinn-support-preparation-final-checks-01.json` and
`runs/owned-djinn-support-preparation-checkpoint-01.json`.
Hosted CI for predecessor `55fc91a` was pending when checked; local checks do not
claim a hosted-CI or full workspace runtime pass.

The owner accepted [source-property preparation over existing Skill occurrences](owned-source-property-preparation-proposal.md).
Detailed wire contracts, exact source/cache evidence and implementation are next;
raw input storage remains on the existing occurrences. Generated saved-usage
ownership and the canonical reference lifecycle remain separate open decisions.

## Checked baseline and original-build results

Use `runs/owned-ice-nova-intrinsics-01/package` as the integration baseline.
Its predecessor is `runs/owned-djinn-tree-grants-01/package`. Publication requires
that exact predecessor and both authenticated Ice Nova intrinsic source reports.
Checked-in authoring is `data/owned/poe2/3887ae68/ice-nova-intrinsics/`.

| Identity | Current value |
| --- | --- |
| Source revision | `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4` |
| Release input | `179b0cd3decbfc79bd0c52ae087597ae5e7a3fc008b35c2638519c5236cf2191` |
| Registry | `8a46cb689f41167a7ed5053b7572e1ff021e4317c49b1892b1219635c0c3dd95` |
| Definitions content | `bf9e59bff312a376f49642ee20846efa21332c8a104fdeed269ab402e5331097` |
| Normalization | `f455fd37ee607baf233ed594ea90e353c2609a0fab98b42e29b3d6b9b6bd1dec` |
| Tree policy | `920c1c1251894ef21baf1dea684359339d33840d2917cbf94844f3e684e52b9f` |
| Schema / operations | V5 / `owned-domain-operations-v17` |

The eighteen package files total **60,757,799 bytes** with 78 provenance rows.
All rebuild byte-identically. One revised Skill declaration, ten allocated IDs,
four local-key tables, one program and the explicit V2 routing adaptation are new;
other values are preserved after checked dependency rebinding. All 110 queries
are unchanged. The registry ends at `32df`.
Mechanics remain Partial and there is no complete evaluation bundle.

| Original | Unchanged saved selection | Selected unresolved issues | Native evaluation |
| --- | --- | ---: | --- |
| 01 | Kelari / Sand Djinn, Kelari's Deception | 113 | Not run: Pending |
| 02 | Twister, skill set 6 | 116 | Not run: Pending |
| 03 | Whirling Assault, average-damage mode | 108 | Not run: Pending |
| 04 | Crossbow Shot | 121 | Not run: Pending |
| 05 | Skeletal Sniper, Basic Attack, skill set 4 | 11 | Not run: Pending |

Crossbow's saved reference selection has no hit-damage output; preserve its
availability result rather than choosing a different skill. Original source
bytes, saved selections, all 110 queries and every unrelated selected obligation
remain unchanged. Local `runs/` files are reproducible evidence, not distributed
game data.

## Next executable work

1. **Complete usage ownership and semantic dispositions for the real selected build.**
   Original05 still has eleven selected obligations. Its preset now has six known
   preferences: four skeletal counts plus Offering and Frost Bomb switches.
   Remaining saved usage includes authored Direct and generated contexts. The
   proposed [generated-skill applicability contract](owned-generated-skill-usage-proposal.md)
   is awaiting the owner; do not relax Core's strict supplying-preset scope first.
   Once accepted, specify/version complete and draft persistence, checked global
   roots and data-aware parameter validation before composition can certify
   nonselection. Keep one native usage request and execution path.

   Source disposition and exact provider correspondence can proceed independently.
   Tree Djinn now has reviewed Allocation supply and raw-level programs; Firebolt
   already uses its exact ItemModifier grant. Finish their saved input/usage
   correspondence; membership classification alone is insufficient. Generated
   quality 12.5 survives source reconstruction, so a provider-wide zero is invalid.
   Review enabled/count/group override/Full DPS consumers across all nine authored roots
   and generated contexts before closing the usage inventory. Unknown settings,
   stale providers and absent declarations remain obligations. Do not import a
   count-one default or treat reporting switches as irrelevant because current
   component metrics do not read them.

   Configuration roles still link 237 origins. Saved config, defaults, repeat/
   averaging and child activation require separate semantic accounting; more
   numeric controls cannot certify an empty scenario-usage inventory. Reuse the
   audits in `runs/owned-after-membership-next-blocker-audit-01.md` and the generated
   usage proposal. Retain the canonical-reference lifecycle decision separately.

   The ten-support preparation sidecar and native admission proof are now checked.
   Existing Direct receiving anchors represent Command, Actor and child paths;
   no new target kind is required. Runtime definitions cover Bidding II/III,
   Magnified Area I/II, Muster, Frost Nexus, Prolonged Duration II, Hulking Minions,
   Kurgal's Leash and Rapid Casting II. All Original01 and Original05 contexts are
   tested; no unreviewed candidate is filtered out to obtain a pass.
   Integrating this into the real support plan still requires complete owners,
   contributors, effective inputs, stages and receiving artifacts. The standalone
   sidecar does not retire the six target issues or complete those inventories.
   Do not weaken early gates or manufacture an evaluator bundle. Any new
   preparation-scoped completeness contract requires a design discussion.
   Numeric Muster delivery still needs actual parent PersistentMinionTypes
   authority. See [Djinn source evidence](owned-djinn-provider-evidence.md).

   Sniper reservation still needs build-driven parent Action contexts; discover
   them from declared mechanics without inserting hidden reference queries.
   Ice Nova's intrinsic tables and exact Action routing are now published. Connect
   its final-level slot through source-backed input assembly; do not substitute
   raw Gem level or the finite fixture's explicit provider. Prove the exact external
   and admitted-support property contributors before level validation and the
   existing child projection. The selected supports are Encroaching Ground,
   Magnified Area II and Rapid Casting I; archived copies also include Rapid
   Casting II and Astral Projection. Empty observed property lists do not establish
   complete contribution coverage. Quality effects, infusion activation and full
   damage contributors remain unfinished. See
   [minion/spell evidence](owned-minion-spell-input-evidence.md) and
   [reservation](owned-summon-reservation.md).
2. **Integrate readiness with source-backed final-input assembly.** The generic
   V16 contract and public support-plan proof now exist. With the real release
   now on V17, establish exact source-effect membership for physical Gems and
   nonphysical Direct sources, then apply supported properties once per intended
   source. The owner accepted the [source-property relation](owned-source-property-preparation-proposal.md):
   reuse exact existing Skill occurrences, with explicit membership and aggregation
   permissions. Specify/version its wire contracts and obtain the missing Ice
   property-census/cache evidence before adding source-target authority. Include
   actor/query property contributors even when no support is admitted; retain
   checked alias correspondence, stable contexts and complete early owner programs.
   This decision does not authorize a Direct self-parameter writer or relax coverage.
   Djinn source witnesses now show summon/Command membership; native property
   delivery still needs explicit declared authority. A shared ancestor or authored
   support target alone supplies no such authority. Preserve the fractional
   ordering witness `12 + 0.25 + 0.75 -> 13` through final validation. Use existing
   declared-child projection authority; the component proof does not authorize
   arbitrary parent reads or a self-parameter writer. Reuse shared semantic raw
   slots with explicit producer authority; Direct effective inputs use Skill
   derived channels, and only declared generated children receive projections.
   Before early support execution, require checked stagesV2 and receivingV2
   metadata, complete phase inventories and actual/implicit dependency checks
   in the same occurrence graph and attempt budget. Keep final action/query
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
from aliases. Ice and Sniper action selection/physical disposition, plus Sniper count/reservation, have
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
  Djinn raw inputs and complete intrinsic inventories now import into the V5/V17
  release, with declared Command/Actor/child topology. Final input producers,
  support admission and complete numerical coverage remain unfinished.

**Pending owner input:** [generated-skill usage ownership](owned-generated-skill-usage-proposal.md).
The recommendation keeps intent in the skill preset and adds explicit applicability
for exact tree/item providers selected by other build axes. Proven nonselection
can leave a preference dormant; stale, unknown or partial providers remain
obligations. This requires a versioned contract and data-aware validation of all
stored resolved preferences, including overridden and dormant records. Do not
relax strict preset ownership before the owner chooses this model or a separate
combined-variant usage layer. Count import and source evidence work are independent.

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
