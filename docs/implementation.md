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

## Latest checkpoint: three skeletal families and stable input identities

Subsystems changed: owned Import, reviewed definition/normalization data,
optional PoB source observation and Rust tests. Arsonist, Frost Mage and Reaver
reuse the existing singleton-minion correspondence, physical V3 disposition and
shared typed value recipes. Nine ordinary grant programs connect three physical
Gems to their summons, actors and first child actions. There is no new Core/Engine
model, interpreter or native Lua dependency.

All **15 physical occurrences** now have complete intrinsic input lists: three
in Original01 and twelve in Original05, including archived presets. Raw values,
local IDs, allocator watermarks, usage records and real Pending usage obligations
are preserved. Three selected obligations close in each affected original;
selected counts are **115 / 116 / 108 / 121 / 14**. Additional child actions,
Commands, final input producers and numerical mechanics remain Partial.

The five-original preservation gate exposed and fixed an Import allocation-order
bug. A newly admitted earlier V3 disposition used to allocate the preset usage
issue ahead of an existing ordinary consumer, shifting local IDs. A bounded
attachment pass now runs after ordinary Gem/Direct consumers, retaining each
physical issue's original allocation and retiring only its proven obligation.
V1/V2 behavior and persisted wire contracts are unchanged. Fresh V3-only imports
without an ordinary usage consumer can allocate their new Pending usage issue
later; historical fresh-allocation order is not promised for that case.

The shared source witness passes **62 cases in both JIT modes**, with fresh load
and two requested rebuilds retained independently. All fifteen actual source
objects, archived activations and independent MAIN/CALCS choices are covered.
The existing 34-case Sniper reports remain byte-identical. No canonical parity
lifecycle is selected. Source selectors and names remain in Import/data, outside
native gameplay contracts.

Publication compares every field and provenance link in all five original drafts.
All **110 query rows** are unchanged, eighteen package files rebuild exactly,
30 original MAIN/CALCS correspondences resolve, and nine edited-source controls
retain unsupported alternatives as Pending. A shared exact-preservation helper
replaces the duplicated Ice Nova/Sniper test plumbing; both older publications
and their mutation controls pass again.

Validation passes: **519 targeted Import tests**, all six CLI tests including
three real publications, both complete two-mode source tests, strict workspace
and native Clippy, both WebAssembly configurations, compiled boundaries, native
dependency closure and all-package formatting. Fresh predecessor drafts also
match all five historical saved drafts after normalizing only lineage identity.
No full workspace runtime run or hosted-CI pass is claimed. Mechanics remain
Partial, there is no evaluation bundle, and native originals remain **0/5**.

Evidence: `runs/owned-skeletal-inputs-02/validation.json`,
`runs/owned-skeletal-inputs-import-01.log` (191 library and six compact tests;
its later test-only provenance assertion was corrected),
`runs/owned-skeletal-inputs-import-02.log` (remaining 322 passing tests),
`runs/owned-skeletal-inputs-cli-04.log`, and
`runs/owned-skeletal-inputs-source-01.log`,
`runs/owned-skeletal-inputs-final-checks-01.json`, and
`runs/owned-skeletal-inputs-historical-preservation-01.json`.
`runs/owned-skeletal-inputs-02/selected-05-report.json` records the fourteen exact
selected-request obligations. The source reports are under
`runs/owned-skeletal-actor-action-source-01/`.
The earlier `runs/owned-skeletal-inputs-01/` is a failed diagnostic publication,
not an accepted baseline. `runs/owned-skeletal-inputs-checkpoint-01.json` records
validation and Git publication state. See the [source-action contract](owned-source-actions.md),
[source evidence](owned-minion-spell-input-evidence.md) and
[authored packet](../data/owned/poe2/3887ae68/skeletal-inputs/README.md).

## Checked baseline and original-build results

Use `runs/owned-skeletal-inputs-02/package` as the current integration baseline.
Its predecessor is `runs/owned-sniper-inventory-01/package`. Publication requires
that exact predecessor and both authenticated skeletal source reports.
Checked-in authoring is `data/owned/poe2/3887ae68/skeletal-inputs/`.

| Identity | Current value |
| --- | --- |
| Source revision | `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4` |
| Release input | `1d58362e0a808164fa7d554b665c60abb52d943a17f1a4c66cd6b8edb141d42e` |
| Registry | `e1f71fd439c5aee06bd09dedc91f1e166048ce3b430932f720cb098a8dc421b9` |
| Definitions content | `0bc454532c3287bf2b968959e75e5cc2cf1c4192e24a7db7f751c48fa1d66e12` |
| Normalization | `70b2d28c20bd058bbce4af3a7bda8d2b0c00f9b0c4156d7094edd446f45c456e` |
| Tree policy | `33a834a91b5050b88b0abe5b3e7cb8249ed881c23016415e503b24e39ddcae72` |
| Schema / operations | V5 / `owned-domain-operations-v17` |

The eighteen package files total **60,545,467 bytes** with 70 provenance rows.
All files rebuild byte-identically. The registry appends 29 addresses through
`32a1`; old definitions, rules, values and queries survive except the reviewed
three-family topology and checked dependency rebinding. No numerical table,
metric or complete evaluation bundle is added.

| Original | Unchanged saved selection | Selected unresolved issues | Native evaluation |
| --- | --- | ---: | --- |
| 01 | Kelari / Sand Djinn, Kelari's Deception | 115 | Not run: Pending |
| 02 | Twister, skill set 6 | 116 | Not run: Pending |
| 03 | Whirling Assault, average-damage mode | 108 | Not run: Pending |
| 04 | Crossbow Shot | 121 | Not run: Pending |
| 05 | Skeletal Sniper, Basic Attack, skill set 4 | 14 | Not run: Pending |

Crossbow's saved reference selection has no hit-damage output; preserve its
availability result rather than choosing a different skill. Original source
bytes, saved selections, all 110 queries and every unrelated selected obligation
remain unchanged. Local `runs/` files are reproducible evidence, not distributed
game data.

## Next executable work

1. **Resolve the closest real build's Direct inputs and support destinations.**
   Original05's fourteen selected obligations are six support targets, two Direct
   parameter inventories, and one each for scenario usage, preset usage, skill
   membership, support-origin discovery, configuration roles and external
   assumptions. Configuration roles still link 255 source rows; numeric controls
   alone do not close this inventory.
   Manual Sand Djinn (source216, Skill0322) and Water Djinn (source229, Skill032c)
   already have their own raw level20/quality0. Supports217–219 and230–232 need
   those exact respective sources. These ordinals identify fixture evidence,
   never production dispatch. Author reviewed summon-to-Command/Actor/child
   topology through existing Skill supplies, grants and receiving paths, keeping
   Partial mechanics. Preserve manual versus allocated siblings and the catalog's
   non-container proof; do not fabricate a physical Gem or a second Direct root.
   Current DirectV1 accepts flat rows and always retains its parameter inventory
   issue; query/action correspondence requires a physical Gem. Extend those Import
   boundaries with explicit typed Direct source identity and versioned policies,
   preserving old physical locator and DirectV1 contracts. Do not add ambiguous
   optional Gem/Direct selectors. Reuse bounded minion decoding, canonical set
   validation, scalar recipes, exact field accounting and the real containing
   preset's Pending usage proof. Nested maps require an explicit reviewed new
   policy; never weaken the shared physical census to admit them.
   Extend the existing source observer to exercise all eleven original Direct
   occurrences, archived activation and independent selectors/maps in both JIT
   modes and all three lifecycle stages. Existing Djinn evidence establishes
   selected topology and archived-edit isolation, not that broader gate. Resolve
   diagnostic reference selections separately from authored usage. Input-list
   completion, support targets and per-receiver support admission are independent
   gates: a sole Direct root is insufficient to retire the six target issues.
   Require exact source provenance and the existing native support-plan proof for
   summon/Command/child paths, retaining incomplete input/contributor gates.
   Rerun all five originals and preserve their queries at each boundary.
   Sniper reservation still needs build-driven parent Action contexts; discover
   them from declared mechanics without inserting hidden reference queries.
   Ice Nova's constructed per-level stats and additive duplicate aggregation
   remain separate numerical work. See [Djinn source evidence](owned-djinn-provider-evidence.md),
   [minion/spell evidence](owned-minion-spell-input-evidence.md), and
   [reservation](owned-summon-reservation.md).
2. **Integrate readiness with source-backed final-input assembly.** The generic
   V16 contract and public support-plan proof now exist. With the real release
   now on V17, establish exact source-effect membership for physical Gems and
   nonphysical Direct sources, then apply supported properties once per intended
   source. Djinn summon/Command membership needs explicit evidence; a shared
   ancestor or authored support target alone supplies no such authority. Preserve the fractional
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
