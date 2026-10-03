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

## Latest checkpoint: explicit native readiness

Subsystems changed: owned Core/Data contracts and the native Engine compiler.
Operations V16 uses stage V2 metadata to classify required inputs and exact
program output roles. Receiving V2 adds separate preparation applicability and
numeric properties. All data remains injected; this adds no Lua execution,
source parser, second build model or caller-provided final inputs.

The public support-plan proof passes: a physical root supplies a summon and
actor; explicit preparation inputs allow native support admission; admitted
properties feed ordinary actor-to-child final-input assembly; action metrics
then require the final level and quality. Repeated roots and sibling skills use
their existing exact identities. This is a synthetic component proof, not a
source-gem membership or original-build parity claim.

Cold compilation checks all potential early reads and writers, including lazy
branches and unretained support templates. Execution also retains ancestor Skill
requirements after structural activation. Scalar values, contribution streams
and modifier transforms remain distinct channels. Legacy defaults and omitted
wire fields remain unchanged; V16 refuses absent checked readiness metadata.

Validation passed: Core version checks; 57 Data authoring/regression checks;
171 Engine unit tests; 14 public readiness controls; four reuse/budget/parallel
tests; 131 existing Engine integration checks; ten Import release checks; and
three CLI publication checks, including the original-build rebuild. The reuse
proof includes 96 alternating evaluations on four Rayon workers. The compiler
cleanup was followed by another 40 unit and 88 integration checks. Strict
workspace and native-CLI Clippy, both WASM configurations and the compiled owned
module boundary checks passed.

The rebuilt real V15 release has all eighteen files byte-for-byte identical to
the baseline below (60,343,480 bytes), preserving all 110 queries and selected
issue counts. No new game data or source-parity result is claimed. Complete
native original builds remain **0/5**; source-gem membership and final-input
integration remain separate work. The direct SkillUse input decision remains
separate too.

Local evidence: `runs/owned-readiness-checkpoint-01.json`,
`runs/owned-readiness-legacy-release-01/validation.json`, and
`runs/owned-readiness-*.log`. These are reproducible local evidence; the new
Core/Data/Engine Rust tests preserve the component contract in the repository.

## Checked baseline and original-build results

Use `runs/owned-minion-physical-inputs-02/package` as the current integration baseline.
Its predecessor is `runs/owned-skill-usage-inputs-02/package`. The minion `-01`
attempt stopped before publication because authoring supplied the aggregate
release source pin instead of the exact mapping source pin required by V4.
The corrected authoring retains all other source/evidence commitments.

| Identity | Current value |
| --- | --- |
| Source revision | `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4` |
| Release input | `228a5f9bd06415324d0a35e31b51c6bc92701cc83180df03f88b04646556651d` |
| Registry | `b287edcac70c37e5610b5c08ae73c9e9cccde99a0b9746dd1a0a459bd95d685a` |
| Definitions content | `e8a81ab31028ef7cf8c1fdda7f4db2d600e05d04a3289f2c6500b3eacae98758` |
| Normalization | `630f26e196bd02d76709ad6145800405a26fdc9c3d84deb9d1b14b2a48d3205e` |
| Tree policy | `422266748e4ff3b8e801594bc26db01fa7b20514c74ae6d7be8065e3323527cb` |
| Operations | `owned-domain-operations-v15` |

The eighteen package files total 60,343,480 bytes and retain 57 provenance rows.
This release promotes three existing Gem schemas and adds six parameter slots,
with no new program or table. The Partial effect-application registry and absence
of an evaluation bundle are preserved. Complete prior
contents and original draft/sidecar data are compared after accounting for the
explicit new inputs and dependency identities.

| Original | Unchanged saved selection | Selected unresolved issues | Native evaluation |
| --- | --- | ---: | --- |
| 01 | Kelari / Sand Djinn, Kelari's Deception | 116 | Not run: Pending |
| 02 | Twister, skill set 6 | 116 | Not run: Pending |
| 03 | Whirling Assault, average-damage mode | 108 | Not run: Pending |
| 04 | Crossbow Shot | 121 | Not run: Pending |
| 05 | Skeletal Sniper, Basic Attack, skill set 4 | 19 | Not run: Pending |

Crossbow's saved reference selection has no hit-damage output; preserve its
availability result rather than choosing a different skill. Original source
bytes, saved selections and all 110 queries remain unchanged.

Evidence: `runs/owned-minion-physical-inputs-02/validation.json`, its
`execution-receipt.json`, and `runs/owned-minion-inputs-*.log`. Local `runs/` files
are reproducible evidence, not distributed game data; checked-in authoring and
tests preserve the reproduction contract.

## Next executable work

1. **Resolve the closest real build's input contracts.** Original05's nineteen
   selected obligations are seven Gem parameter inventories, six support
   targets, and one each for scenario usage, preset usage, skill membership, support-origin discovery,
   configuration roles and external assumptions. The seven Gem rows carry real
   count/global/action/minion settings; do not declare them empty using the
   SupportAssignment-only absence proof. Arsonist, Frost Mage and Reaver now have
   Known-but-Partial physical schemas and proven intrinsic scalars. Their saved
   minion/action/count/global fields and missing Commands remain separate work.
   Configuration roles link 255 source rows, not merely the numeric controls
   just added.
   **Complete the remaining source-field dispositions.** Persistence, composition,
   native execution and the first source-bound Boolean projection now exist.
   Pain Offering's switch feeds the existing Boolean Skill channel `3227`.
   Finish finite physical/preset inventory proofs only after accounting for every
   saved field and its actual consumer; persistence is not inventory proof. Preserve
   count separately; do not invent a count statistic without a numerical consumer.
   Start with Offering's finite field/consumer inventory; obtain distinct source
   controls for minion usage and Ice Nova's constructed stat sets. Manual Djinn
   inputs need the separately proposed direct SkillUse contract. Rerun all five originals
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

The source audit in `runs/owned-usage-import-next-audit.md` records the exact
seven Original05 Gem rows and every source-field disposition. The completed
singleton occurrence witness covers only Pain Offering and Frost Bomb among
those seven. The four skeletal-minion families now have separate physical scalar
evidence; their action/usage inventory and Ice Nova's constructed stat sets still
need their own evidence. Do not close the scenario usage inventory
merely because skill-preset preferences are now representable.

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

The [direct SkillUse inputs](owned-skill-occurrence-input-proposal.md) and
[socket configuration](owned-socket-configurations.md) proposals also remain
unaccepted. An answer to the earlier actor/support/finite-stage questions or the
general effect-application model does not implicitly accept these later changes.
Proceed with the accepted usage and readiness contracts and independent cleanup.

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
