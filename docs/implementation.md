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

## Latest checkpoint: generated Djinn supply and independent source evidence

Subsystems changed: reviewed owned data and Rust validation, plus optional PoB
source witnesses. No production Core/Engine/Import contract changes. The two
existing Djinn ascendancy passives now have authored supply/grant declarations
and ordinary programs projecting source-defined raw level 1 into the existing
shared typed slots. There is no additional skill model, Gem or evaluator.
Publication passes with all eighteen files rebuilding byte-identically and all
five saved selections preserved. Three native component tests also pass.
**0/5 native builds are complete.**

Generated quality deliberately has no producer. The new complete-source witness
proves matching saved generated entries retain quality 12.5; zero is only a
constructor default when PoB creates a replacement entry. A universal zero
projection would change valid builds. The accepted shared-slot authority model
is retained. Saved provider-bound input storage, usage applicability, effective
inputs and support admission remain separate work.

The generated-setting witness passes 46 cases in each JIT mode, including all
five originals, archived selections and controls for enabled/count/group/global/
Full DPS/quality settings. It retains fresh and two rebuilt states and exact
provider/source-object joins. Reporting rows without source identity remain
explicitly unjoined. The existing 34-case membership witness was rerun after its
small test-harness extension; both previous report files remain byte-identical.

The independent Ice Nova intrinsic witness also passes in both JIT modes:
12 loads, three lifecycle stages each, with all 80 level/stat-set combinations
and 36 boundary probes at each stage. It calls the original stat assembler.
Duplicate stat entries add, including radius 32 + 16 = 48. Invalid helper levels
fall back to row 1; that source behavior is not native admission authority.
This establishes the next numerical data conversion, not complete damage parity
or a real final-level producer. Original non-main Ice Nova outputs remain unavailable.

The finite native fixture loads the actual published rule bodies and required
input schemas under V17. It proves Allocation provider identity and raw Count 1,
independent manual occurrences, missing-quality execution gates, exact provider
removal and loadout behavior. Full reports agree across A/B/C/A scratch reuse,
48 Rayon attempts and recovery after work exhaustion. Actual Partial owners are
rejected by the early readiness check, and the real Shared ascendancy pool rejects
loadout-scoped allocations. The constant test consumer demonstrates readiness,
not Djinn damage or support parity. No published coverage was marked Complete.

Evidence and exact report hashes are in [Djinn provider evidence](owned-djinn-provider-evidence.md),
[minion/spell evidence](owned-minion-spell-input-evidence.md) and the
[tree grant packet](../data/owned/poe2/3887ae68/djinn-tree-grants/README.md).
Source logs: `runs/owned-generated-skill-usage-source-02.log`,
`runs/owned-generated-usage-membership-compat-01.log`, and
`runs/owned-ice-nova-intrinsic-source-01.log`. No canonical reference lifecycle
has been selected; all witnesses preserve the three stages independently.

Validation passes: two CLI tests (including publication/five-original preservation),
three native component tests, both new complete-source witnesses and the unchanged
membership replay. Strict workspace and native Clippy, both WebAssembly library
configurations, compiled boundaries, native dependency closure and all-package
formatting pass. No full workspace runtime or current-head hosted-CI pass is claimed.
The earlier hosted run `37146784579` at `8f3844e` completed all eight jobs successfully;
that is separate from these local checks and later CI runs.

Evidence: `runs/owned-djinn-tree-grants-01/validation.json`,
`runs/owned-djinn-tree-grants-cli-01.log`,
`runs/owned-djinn-tree-grants-native-04.log`,
`runs/owned-djinn-tree-grants-final-checks-01.json` and
`runs/owned-djinn-tree-grants-checkpoint-01.json`. Native attempts `-01` and `-02`
are retained fixture diagnostics (trait spelling and support-quality unit).
The repairs changed only test construction; no production gate or schema changed.

## Checked baseline and original-build results

Use `runs/owned-djinn-tree-grants-01/package` as the integration baseline.
Its predecessor is `runs/owned-skeletal-counts-03/package`. Publication requires
that exact predecessor and both authenticated Djinn provider source reports.
Checked-in authoring is `data/owned/poe2/3887ae68/djinn-tree-grants/`.

| Identity | Current value |
| --- | --- |
| Source revision | `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4` |
| Release input | `82aa58ffa676aa4c038392395b618e5506a351c6c8679f19add3fd58ffd5068d` |
| Registry | `80ce6e749ad01b95b02e827a312010b7ad5e50ba49269dd78e603a5e151a5a91` |
| Definitions content | `fb5260ade7a70e6fc1490ab039cf91880d86107a95af31ca6cacdb1c0b8d221c` |
| Normalization | `9444c6fef0ba5b0b54a307d2a8b999b04a636b1e3613b61def81cdc5cd64e6e4` |
| Tree policy | `f77de96cbef26750d9e31e6c9ebcece64aa6119ea38e46a914981a83f4d62379` |
| Schema / operations | V5 / `owned-domain-operations-v17` |

The eighteen package files total **60,712,832 bytes** with 76 provenance rows.
All rebuild byte-identically. Two passive declarations, four supply/grant slots
and two source-defined programs are new; other values are preserved after checked
dependency rebinding. All 110 queries are unchanged. The registry ends at `32d5`.
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

   Keep support admission as the following measured blocker. The four actual
   Djinn supports are Bidding II, Magnified Area I, Muster and Frost Nexus.
   Existing Direct receiving anchors represent Command, Actor and child paths;
   no new target kind is required. Capture initial skill/minion types and flags
   separately from post-admission additions, then use reviewed preparation data
   and the existing native admission component. That component alone cannot
   retire the six target issues: the real support plan rejects incomplete owners
   and contributors before its prefix. Do not weaken completeness or fabricate
   a complete release. Any new preparation-scoped completeness contract would
   require a design discussion. These four supports have no level/quality
   GemProperty bonus; general source-property aggregation is not a prerequisite
   for their bounded admission proof. Numeric Muster delivery still needs actual
   parent PersistentMinionTypes authority. See [Djinn source evidence](owned-djinn-provider-evidence.md).

   Sniper reservation still needs build-driven parent Action contexts; discover
   them from declared mechanics without inserting hidden reference queries.
   Ice Nova's full intrinsic source witness now passes. Convert the two existing
   stat sets into injected tables/additive constants and exact Action routing,
   with an explicit validated final-level input whose real producer stays Pending.
   Native intrinsic parity and integration remain the next numerical work. See
   [minion/spell evidence](owned-minion-spell-input-evidence.md) and
   [reservation](owned-summon-reservation.md).
2. **Integrate readiness with source-backed final-input assembly.** The generic
   V16 contract and public support-plan proof now exist. With the real release
   now on V17, establish exact source-effect membership for physical Gems and
   nonphysical Direct sources, then apply supported properties once per intended
   source. Djinn source witnesses now show summon/Command membership; native property
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
