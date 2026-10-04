# Implementation plan and resume point

Updated: 2026-10-04 (EDT).

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

## Latest checkpoint: ordinary Minion-level Amulet copy

Subsystems changed: owned game-data artifacts and Rust verification. No production
Rust, public contract, raw-input slot, item parser or interpreter changed. The
existing Minion modifier now has a separate per-occurrence copy program; the
original direct contribution is preserved, including when its copy is zero.

Two new Stats `32e3..32e4` hold equipment eligibility and the required pre-Amulet
percentage snapshot. Eight ordinary programs add six typed eligibility facts,
the per-record copy and Mystic Attunement's actual 25% contribution.
Copies divide the percentage by 100, bypass scaling for identity/unscalable
records, and otherwise floor each scaled record individually. The copy does not
read its own output into its factor. No snapshot aggregator or complete incoming
inventory is supplied: missing data stays unresolved, never an implicit zero.

Lapis/Solar Amulets now have Complete Amulet-only placement; Iron Crown/Kamasan
Tiara have Complete Helmet-only placement. All four reject incoming socket
placements while retaining their own existing augment sockets. The proved
non-Amulet types of Rattling Sceptre/Sacred Focus establish false copy eligibility
independently of their still-Partial conditional weapon placements. Every other
descriptor field, owner closure and existing program is preserved exactly.

The source receipt reuses both JIT modes of the prior Minion and property
witnesses: 34 normalized source pins and 876 exact pointers, plus reviewed static
placement/passive facts. Optional publication rechecks the pinned files, all six
exact base blocks and the exact tree-node excerpt; it does not treat an ignored
derived report as an original runtime oracle. No source runtime was rerun. The
old Minion reports have identical substantive observations but differ in their
references to prior mode-specific evidence; their whole bytes are not claimed equal.

All **11 selected Rust tests** pass: one authoring test, one full publication/
original-preservation test and nine native tests. The native fixture reuses the
existing ordinary Minion evaluator with exact authored bodies. It proves selected
Crown/Solar Minion values 1+1 plus distinct copy 0, per-record 1/3 rounding at
0%, 25%, 50% and 100%, the actual 25% passive, explicit unscalable branch
controls, wrong slot/scope/socket rejection, non-Amulet lazy inactivity, exact
missing-producer/Partial failures, A/B/A and 48 Rayon evaluations. Its finite item,
catalyst and passive boundaries do not certify full items or allocation legality.
It retains the existing ordinary-operations fixture; publication separately checks
the real schema V5/operations V18 package.

All **15 strict checks** pass: workspace/native Clippy, both WebAssembly library
configurations, compiled boundaries, native dependency closure and eight package
format checks. Native execution still has no PoB/Lua dependency. This is targeted
validation, not a full-workspace runtime or hosted-CI claim. Evidence:
`runs/owned-amulet-level-copy-tests-02.json`,
`runs/owned-amulet-level-copy-final-checks-01.json`,
`runs/owned-amulet-level-copy-02/validation.json` and
`runs/owned-amulet-level-copy-checkpoint-01.json`.

Both validation attempts pass; the second strengthens static-source authentication
and exact missing-producer assertions. All five originals retain exact local IDs,
saved inputs, selections and 110 queries. Their selected obligations remain
**113/116/108/121/11**, with no complete evaluation bundle. The Sniper's selected
Minion item rolls were already imported; this fixes a later numerical gap without
claiming an input-issue reduction. **0/5 complete native builds.**

## Checked baseline and original-build results

Use `runs/owned-amulet-level-copy-02/package` as the integration baseline.
Its predecessor is `runs/owned-ice-nova-source-inputs-01/package`. Publication
requires that exact predecessor and the authenticated item-property/static
source evidence. Checked-in authoring is `data/owned/poe2/3887ae68/amulet-level-copy/`.

| Identity | Current value |
| --- | --- |
| Source revision | `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4` |
| Release input | `f604070539924fd7961becce302d0db5ae49d683a88bdb73c57ed59e0c63b809` |
| Registry | `93a3f8f3bfeda549fecdd367695a3e070e95c0a4bb0fc86af35e152236467d2f` |
| Definitions content | `d5b8da81616358e43d77654619503246d6adf7bf47cdf0442740c1d0adde3f7f` |
| Normalization | `cc764d61e3416fc36378c5d34fd4e094ab1d7e7d43826a9a06064ff087e79c34` |
| Tree policy | `45dd0f39df33159be48a9f3a461710e8e6f9fe5b249abf15b20828ebfc359b52` |
| Schema / operations | V5 / `owned-domain-operations-v18` |

The eighteen package files total **60,790,891 bytes** with 82 provenance rows.
All rebuild byte-identically. Two Stats, four placement refinements and eight
program fragments are new; all unrelated schema/rules/import fields survive exact
checked dependency rebinding. All 110 queries are unchanged. The registry ends at
`32e4`. Mechanics remain Partial and there is no complete evaluation bundle.

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

1. **Complete actual property contributors and owner coverage.** The ordinary
   Minion-level Amulet copy and finite placement facts now have authored fragments.
   Next establish the pre-Amulet snapshot's actual contributors and aggregate
   authority, then join the existing item arithmetic into source preparation.
   Audit the selected Sniper request first: its two Minion item occurrences are
   already canonical. Neither absence of explicit Mystic Attunement nor an
   observed zero proves all implicit, item, configuration or earlier-copy paths
   absent. Earlier Kalandra/Quiver copies can affect the general snapshot. The
   real 50% source control is a four-member rings-and-amulets family;
   implement its full owned meaning and source/numeric admission before claiming
   a complete factor item. Keep its own copy out of the pre-Amulet aggregate.

   Keep ordinary `GemProperty` before support admission and actor-provided
   `SupportedGemProperty` after census. Named unkeyed supported records retain
   their value even at zero copy scale, unlike keyed level records. Spell/Nova/
   Fire item families and the actual Original04 Rune source need separate proofs.
   No unchanged original contains the plain untagged Spell-level family; do not
   add it merely to improve a controlled Ice case while a real selected blocker
   has a clear fix. Evidence: `runs/owned-item-property-import-audit-01.md`,
   `runs/owned-amulet-copy-scope-audit-01.md` and the checked packet README.

   Prove complete real external/support inventories and all relevant owner
   programs before publishing receiving V3/stages V3 or claiming a working original.
   The operations V18 Partial release does not weaken that gate. Establish real
   admission predicates and supporting mechanics for all six observed Ice support
   candidates; the finite arithmetic component's already-admitted positions are
   not that proof. Supported/final quality and broader ordinary property families
   remain explicit. Validate nonzero real item/support mutations, independent
   sources, exact requests and unchanged originals at the next checkpoint.

2. **Complete usage ownership when the pending owner answer arrives.** Original05
   still has eleven selected obligations. Six preferences already import: four
   skeletal counts plus Offering and Frost Bomb switches. Remaining authored
   Direct/generated usage needs the [generated applicability contract](owned-generated-skill-usage-proposal.md).
   Do not relax Core's strict supplying-preset scope in advance. Version the chosen
   model, validate every stored preference including overridden/dormant records,
   and preserve unknown source fields and all existing query identities.
   Tree Djinn and item-granted Firebolt still need exact saved-input/provider
   correspondence. Generated quality12.5 survives source reconstruction; a
   provider-wide zero is invalid. Account for enabled/count/group/Full DPS settings
   across authored and generated contexts without count-one defaults. The 237
   configuration origins still need semantic usage accounting; additional numeric
   controls alone cannot certify an empty scenario-usage inventory.

3. **Connect the remaining numerical consumers.** Real source-property
   integration is step 1. Existing minion-level item producers alone do not prove
   completeness of incoming properties or all final inputs.

   The Djinn preparation sidecar already proves ten supports and 1,296 admission
   contexts, including Original01. Integrate it only with complete real owners,
   inputs, stages and receiving metadata. It does not itself retire the six
   support-target issues. A new preparation-scoped completeness model would need
   a design discussion. Muster still needs actual parent PersistentMinionTypes
   authority. Sniper reservation needs build-driven parent Action contexts without
   hidden reference queries. Quality, infusion, Gigantic, physical-range damage
   and complete offence/defence follow their measured input dependencies. Finalize
   and evaluate the exact unchanged request before claiming a working build.

4. **Continue bounded retirement where files do not overlap.** Remove legacy
   paths together with exclusive scaffolding after their named consumers retire.
   Separate shared skill identity and Lua-number formatting helpers needed by
   owned conversion before removing profile preparation. Retain independent
   numerical references and acquisition consumers; do not add compatibility
   facades for APIs whose consumers have already disappeared.

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

- **Accepted:** [source-property preparation](owned-source-property-preparation-proposal.md),
  reuse existing Skill occurrences with exact source membership and aggregation
  permissions. Bounded original-source evidence and the executable native relation
  component pass. Complete real property producers and final assembly remain open.

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
