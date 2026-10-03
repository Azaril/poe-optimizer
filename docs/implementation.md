# Implementation plan and resume point

Updated: 2026-10-02 (EDT).

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

## Latest checkpoint: typed inputs on exact skill occurrences

Subsystems changed: owned Core persistence/binding, Data schema validation and
native Engine compilation. The owner accepted shared typed slots with explicit
producer authority. A Direct SkillUse now carries its own typed raw values;
provider-generated instances of the same definition receive their values through
checked projections. Authored-only, projected-only and shared permissions apply
at the exact occurrence. Physical Gem uses cannot carry the Direct input layer.
Raw and final input identities remain distinct, and no ancestor fallback or
Direct computed-parameter writer was added.

Schema V5 and operations V17 opt into this contract; defaults and current game
data remain unchanged. V17 retains V16's explicit preparation/execution readiness.
Omitted fields preserve historical bytes; explicit null, wrong producers,
duplicate writers and incompatible operation versions reject. Required values
and unresolved inventories retain their existing coverage/availability gates.

The native proof exercises two Direct uses plus a generated sibling, independent
input edits, missing child producers, inactive projections and required-domain
partitioning. A→B→A scratch reuse, a failed bounded attempt and 48 evaluations on
four Rayon workers preserve complete reports. Core, Data, Engine and Import
regression targets pass. These are component checks, not Djinn numerical parity.
The shared test fixture now constructs and validates each requested version once,
removing its incompatible intermediate legacy compilation. No production legacy
closure was removed in this checkpoint.

Two optional complete-source witnesses pass in both LuaJIT modes. Minion count
feeds reservation while Full DPS has separate aggregation; global switches are
inert in the witnessed skeletal domain. Ice Nova has two constructed stat sets;
extra metadata aliases are not additional sets. The
[source evidence](owned-minion-spell-input-evidence.md) records the finite scope,
reproduction and next native consumers. The optional reference still has no role
inside native evaluation or a Rayon worker.

Validation passes: 169 Core, 32 Data, 55 Engine and 274 Import checks, two CLI
publication checks, two complete-source witnesses and two evidence-helper tests.
Strict workspace/native-CLI Clippy, both portable library configurations,
owned compiled-module boundaries, native runtime dependencies and package
formatting pass. The existing optional Windows CRT linker warning remains.
All eighteen release files are byte-identical to the prior checked baseline;
all ten original draft/sidecar snapshots also agree after normalizing only random
lineage and its dependent draft digest. All 110 queries and selected obligations
remain unchanged. No full workspace runtime or current-commit hosted-CI pass is
claimed by these local checks.

Evidence: `runs/owned-skill-inputs-checkpoint-01.json`,
`runs/owned-skill-inputs-compatibility-01/validation.json`,
`runs/owned-skill-inputs-historical-compatibility-01.json` and
`runs/owned-skill-inputs-*.log`. Real Djinn import/data authoring remains next.
Complete native original builds remain **0/5**; the new input contract alone
retires none of Sniper's eighteen selected obligations.

## Checked baseline and original-build results

Use `runs/owned-active-gem-inventory-01/package` as the current integration baseline.
Its predecessor is `runs/owned-minion-physical-inputs-02/package`. Publication
requires the exact predecessor and the authenticated Offering occurrence witness.
The physical proof retains all previous support rows and the existing usage
recipe; it adds no schema, rule, table or scalar value.

| Identity | Current value |
| --- | --- |
| Source revision | `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4` |
| Release input | `9d1bc37d2b119d37c87bb2542e4bdbb6e0c46415677864afc4dbcc49413f35ec` |
| Registry | `b287edcac70c37e5610b5c08ae73c9e9cccde99a0b9746dd1a0a459bd95d685a` |
| Definitions content | `e8a81ab31028ef7cf8c1fdda7f4db2d600e05d04a3289f2c6500b3eacae98758` |
| Normalization | `e9872215f4654b54a51bf06092d03d8d7ddd43066d7669ab11241a3401d375a0` |
| Tree policy | `9ce5aefee7c45bb32e97eb251edcd26e53636d3d8902bc5b2d85daaa432621a4` |
| Operations | `owned-domain-operations-v15` |

The eighteen package files total 60,344,760 bytes and contain 58 provenance rows.
Compared with the predecessor, only normalization, its tree-policy commitment
and release provenance changed. This checkpoint changes none of those bytes.
The Partial effect-application registry and absence of an evaluation bundle are
preserved. Operations remain V15; the implemented V16 readiness and V17 occurrence-input
components still await real source-backed final-input integration.

| Original | Unchanged saved selection | Selected unresolved issues | Native evaluation |
| --- | --- | ---: | --- |
| 01 | Kelari / Sand Djinn, Kelari's Deception | 116 | Not run: Pending |
| 02 | Twister, skill set 6 | 116 | Not run: Pending |
| 03 | Whirling Assault, average-damage mode | 108 | Not run: Pending |
| 04 | Crossbow Shot | 121 | Not run: Pending |
| 05 | Skeletal Sniper, Basic Attack, skill set 4 | 18 | Not run: Pending |

Crossbow's saved reference selection has no hit-damage output; preserve its
availability result rather than choosing a different skill. Original source
bytes, saved selections and all 110 queries remain unchanged.

Evidence: `runs/owned-active-gem-inventory-01/validation.json` and
`runs/owned-active-inventory-*.log`. Local `runs/` files
are reproducible evidence, not distributed game data; checked-in authoring and
tests preserve the reproduction contract.

## Next executable work

1. **Resolve the closest real build's input contracts.** Original05's eighteen
   selected obligations are six Gem parameter inventories, six support
   targets, and one each for scenario usage, preset usage, skill membership, support-origin discovery,
   configuration roles and external assumptions. The six Gem rows carry real
   count/global/action/minion settings; do not declare them empty using the
   SupportAssignment-only absence proof. Arsonist, Frost Mage and Reaver now have
   Known-but-Partial physical schemas and proven intrinsic scalars. Their saved
   minion/action/count/global fields and missing Commands remain separate work.
   Configuration roles link 255 source rows, not merely the numeric controls
   just added.
   **Complete the remaining source-field dispositions.** Offering's physical
   proof now accounts for its intrinsic fields and leaves count/reporting in the
   actual Pending preset usage inventory. Its Boolean switch already feeds Skill
   channel `3227`. The four skeletal families' minion selectors, MAIN/CALCS
   action maps, count and global consumers now have a passing complete-source
   witness, as does Ice Nova's constructed stat-set correspondence. Use these
   facts to author exact typed input/usage destinations. Frost Bomb and Ice
   Nova's primary Skill schemas remain Unmapped. Do not widen Offering's singleton
   frame by merely allowlisting these fields. Preserve exact typed destinations
   and complete inventory proofs; do not invent a count statistic without a
   numerical consumer. Manual Djinn
   inputs now have an accepted and tested shared typed Direct SkillUse contract.
   Implement the exact source-backed Direct import and data declarations next.
   Existing catalog mappings identify the manual source tuples, but their primary
   Skill schemas are Unmapped. Add an explicit optional source policy before the
   normalizer's physical-source rejection and retain the containing skill preset.
   Existing physical Gem scalar/usage policies do not authorize Direct input
   writes. Keep raw/final identities and manual-versus-allocated paths distinct;
   retain Pending support targets until their actual topology is representable.
   Direct materialization also needs an exact non-container payload proof; the
   existing physical-only payload policy must not silently accept a new source
   domain. Record any newly exposed Pending obligation honestly.
   Rerun all five originals
   at each boundary; keep Twister and Sniper as contrasting integration cases.
   Do not replace missing semantics with observed constants or relax coverage.
2. **Integrate readiness with source-backed final-input assembly.** The generic
   V16 contract and public support-plan proof now exist. Before migrating the
   real release, establish exact physical source-gem/effect membership and apply
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
from aliases. Typed native consumers and checked source projection still need
implementation; none of this evidence closes scenario usage by itself.

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
  persistence, binding and native reads pass component tests; real Djinn import
  remains the next integration step.

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
