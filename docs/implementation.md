# Implementation log and resume point

Last updated: 2026-09-08

Active implementation checkpoint: **typed native candidate evaluation**, starting from
clean `f2985be`. Implementation, full integrated validation and isolated release measurements pass;
publication is the remaining checkpoint step. The previous
goal turn made verified progress by delivering schema-4 support loadouts. The full
implementation-plan/native-parity goal remains active.

Native support is still limited to the documented Spark/Mace profiles. The supplied
minion build, general equipment/skill/modifier pipelines and full PoB parity remain
unfinished. This performance checkpoint does not expand admitted game mechanics.
PoB source revision, full source snapshot, supplied originals and six independent goldens
remain fixed. The next source-audited coverage slice is local weapon-modifier assembly.

This is the living record of delivery order, implemented behavior, validation, unresolved
work, and the next session's starting point. [Design](design.md) defines the intended system
and acceptance criteria; [execution and interfaces](execution-and-interfaces.md) and the
[calculation boundary decision](calculation-boundary.md) define
runtime and application contracts. The [PoB investigation](pob-integration.md) and
[prior-art review](prior-art-and-product-review.md) retain dated evidence and rationale.
Update this document at progress checkpoints, rather than adding implementation status to
the design documents.

## Resume here

1. Inspect `git status --short --branch` and the current validation/publication table below.
   Read [native data packages](native-data.md), the [source-extraction guide](game-data-extraction.md),
   and the [data-boundary decision](game-data-boundary.md).
   Runtime injection, numeric configuration, pinned source extraction and dataset-bound
   controlled search are implemented. Check publication status in the current table below.
   Also read [native backend](native-backend.md),
   [native calculations](native-engine.md), [live passive coverage](passive-coverage.md)
   and [tree projection](tree-projection.md). The production target is a fully native
   parallel evaluator; PoB is an optional explicit reference, never a hidden fallback.
2. Keep PoB at `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4` and unmodified. Preserve supplied
   original exports and independent Spark/Mace goldens. Maintain actual-source interpreted
   and warmed parity, full-build differential cases and strict unsupported-mechanic errors.
3. Inspect the current and preceding checkpoint publication status below, and resolve
   any hosted failures before the next expansion.
   The agreed bounded composition is implemented: selected data, shared typed
   class/tree resolution, composed finite graph/catalog, exact source spans, independent
   locks and caller 0/1 ordinary-point / zero/one ascendancy-point budgets. Legacy fixed Warrior
   problems remain supported. Never infer an available point budget from observed counts.
4. Finish current publication gates, then implement the local weapon-modifier slice below.
   Preserve typed/document equivalence and fresh counted finalist checks. Preserve physical and
   effective node identity, selected-data requirements and strict unknown-mechanic rejection.
   Broader source compatibility remains a separate D4 gate; review that contract before
   admitting another upstream revision. Keep the source pin and independent goldens stable.
5. Broaden source-derived passive/modifier extraction beyond the currently admitted
   stat strings and typed numeric modifier operations, with actual-source and full-build parity
   before relaxing admission. Preserve the data bundle's distinction between retained and
   excluded records; schema-1/2/3 packages must be regenerated for schema 4. Connect broader
   equipment, support and supporting-skill catalogs together, keeping multiple required
   skills/items, cross-class search, exact locks and ownership/resource/point accounting.
   No selected special node, grant, socket or unknown mechanic may disappear during projection.
   Global extraction uncertainty remains separate from unsupported selected mechanics.
6. Replace closed profiles with reusable complete native pipelines as coverage permits:
   item/gem/passive modifiers, resolved actors/conditions and resources, offence/defence,
   conversions, ailments, triggers and minions. PerStat/StatThreshold programs now support
   ordered numeric dependencies; actor-target multipliers, recursive producers and special
   GetStat names remain explicit gaps. No profile count or shared trait establishes full
   game replacement. Source/data upgrades require source identity and parity review.
7. Specialize coordinated proposals with game-aware repair and diverse archives. Preserve
   exhaustive tiny references, deterministic one/many-worker comparisons, locks and empty
   sample retry bounds. Benchmark random, greedy and alternating baselines at 5, 15 and 30
   minutes in mapping/bossing before making optimizer-quality claims. Current finite
   Mace search is a diagnostic integration test, not that benchmark or the first product.
8. Profile preparation/calculation/result costs on realistic supported native builds before
   tuning. Reuse immutable prepared inputs, compiled modifiers and task-local scratch state;
   retain the XML-free typed Mace path and extend equivalent preparation to broader native
   pipelines. Catalog hashing, owned measurement conversion and whole-search costs remain.
   Keep versioned result/provenance contracts and fresh native finalist recalculation.
9. Add shared memory/CPU admission, CLI signal cancellation, progress events, checkpoints,
   persisted cache and throughput scheduling. Optional PoB workers retain separate supervision;
   their fresh-process/reset requirements do not apply to native execution. Every dispatched
   calculation, preparation evaluation and finalist check remains in the shared budget ledger.
10. Add offline comparison/HTML reports, then GUI and browser bindings. Portable compilation
   does not prove browser execution, browser parallelism or performance. The browser host
   supplies clocks/scheduling and uses the same calculation contracts.
11. Update this resume point and checkpoint evidence before the next handoff.

For targeted data questions, the user supplied PoE2DB/PoEDB; see the
[lookup constraint](design.md#external-lookup-references). Do not scrape or bulk-extract either site.

No product-scope answer blocks these tasks. Exact objective metrics, thresholds, scales,
required skill/item subsets and encounter assumptions remain explicit per-run inputs.
Assessment reports constraint evidence and primary availability; it does not certify build
legality or turn diagnostic calculation output into a recommendation. All calibration cases
compare independent hosts/extractors using shared PoB calculations, not independent game models.

## Typed native candidate evaluation - 2026-09-08

Starting point: clean `f2985be`. This implements the previously planned typed-candidate
slice below without changing package, tree, source pin, independent goldens, candidate
identity inputs or report schema versions. Current validation evidence is updated before
publication; no pending check is recorded as successful.

| Checkpoint | State |
| --- | --- |
| Import boundary | `NativeMaceComponents` and opaque `NativeMaceCandidate` handles require a strict catalog and fresh bound native baseline. Exact membership, requirements and private component binding reject forged/foreign candidates. Caller locks and point budgets stay in the canonical domain. Parsed weapons, resolved trees and canonical loadouts are shared per axis. |
| Native preparation | `PreparedMaceCandidates` retains numerical axes, selected metrics, private binding and shared injected compiled data. It retains no XML or cached candidate result. Pure calculation, stack snapshots and timed evaluation are separate from allocating owned metric conversion. Per-tree custom-data composition errors are deferred to affected evaluations. `SharedBackend` explicitly shares one backend between document and typed paths. |
| Search integration | Native defaults to `--native-evaluation typed`; `document` retains the comparison path. Baseline and reserved finalist always use fresh complete document calculation and exact realization checks. The generic `CandidateEvaluator::verify` default delegates to `evaluate`; the reserved dispatch retains its existing ledger, deadline, cancellation, panic and consistency checks. Export adds no calculation. Reports add path/preparation evidence, including axis footprint and admitted handle count. |
| Differential validation | All **1,884 legal** candidates in the **2,940-state** catalog agree between typed and full native document paths, including a second injected-data matrix. The other **1,056** fail requirements. Tests cover foreign bindings, exact selectors/availability, signed finite bits, budget/deadline errors and one custom-data pair that fails only its affected tree. Successful calculation/snapshot/timed evaluation performs **zero allocations over 8,000 mixed calls**; owned measurement conversion is separately shown to allocate. |
| Search validation | New generic verification-hook tests and CLI typed/document tests cover serial/Rayon archives, exact finalist XML and dataset companions, tight budgets, empty/infeasible/unavailable domains, warning equality and the optional PoB path. |
| Integrated validation | **421 workspace tests pass, zero failures**, with nine ignored child helpers exercised by their parents. **56 native-only CLI tests**, workspace/native-only Clippy, formatting, dependency isolation and five portable WASM libraries pass. The benchmark's later mode-selection change separately passes Clippy. Evidence: `runs/typed-test-coverage.json` and `runs/typed-{workspace-tests,native-only-tests,workspace-clippy,native-only-clippy,benchmark-clippy,wasm,fmt}.log`. |
| Release reproduction | Typed/document searches at **1/2/4/32 workers**, three repeats each, agree on feasible/infeasible archives, exact finalist XML and selected data. Full domain uses **1,886** attempts. Custom signed resistance/support data has matching infeasible archives and a three-attempt locked export; empty domains spend zero and partial runs stay within budget. Fresh exported evaluations and companion hashes agree. Evidence: `runs/typed-release-check/summary.json`. |
| Static preservation | UTF-8 and **264 local file / 24 heading links** pass across **32 Markdown files**. Source pin/full snapshot, schema-4 package, tree, original exports, dependency manifests and independent goldens remain unchanged. Evidence: `runs/typed-static-audit.json`. |
| Publication | Not yet committed or pushed for this checkpoint. |

See [typed candidate evaluation](native-candidate-evaluation.md) for ownership, APIs,
allocation scope and benchmark reproduction. Eager catalog materialization/hashing and
owned scheduler measurements remain costs; these measurements cannot establish general
build speed or mapping/bossing optimizer quality.

### Release performance evidence

Measured on Windows x86-64, **AMD Ryzen 9 9950X3D, 16 physical / 32 logical cores**, after
local test/build jobs completed. The harness rotates all **1,884 legal mixed candidates**,
with one baseline and **3,768** explicit pre-timing DPS-equivalence calculations. Every
selected mode/worker/repeat has the same checksum within its invocation. No result cache
is used. Preliminary samples during tests are excluded.

Full document and prepared-result rows below use **20,000 evaluations × three repeats**.
The inexpensive layers use a separate **1,000,000 evaluations × five repeats** invocation
to reduce short-sample noise. Values are median evaluations/second, rounded. The pure
calculation layer omits metrics/deadlines; the result-producing layers perform different
amounts of work. These are bounded Mace API measurements, not realistic build throughput.

| Workers | Full document | Prepared full result | Pure calculation | Timed typed snapshot | Typed + owned metrics |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1 | 20,191 | 49,723 | 16,000,691 | 5,533,523 | 4,217,961 |
| 2 | 22,796 | 59,454 | 19,336,376 | 7,882,301 | 5,559,769 |
| 4 | 32,976 | 77,356 | 38,791,415 | 15,744,238 | 11,228,020 |
| 32 | 125,360 | 297,838 | 239,142,912 | 99,195,524 | 74,545,089 |

At one worker, the timed typed-snapshot rate spans **5.48–5.56 million/s** across the longer
samples; at 32 workers it spans **94.3–104.3 million/s**. Some 32-worker numerical samples
still last only 4–15 ms, so their ratios are not sustained-load scaling guarantees.
Raw distributions and setup evidence: `runs/typed-benchmark-isolated.json`,
`runs/typed-benchmark-fast-isolated.json` and their `-summary.json` companions.

In the first isolated invocation, catalog preparation takes **26.63 ms**, numerical axis
preparation **0.079 ms**, and legal-handle construction **6.89 ms**. The prepared numerical
object accounts for **16,983 bytes**, **4 weapons / 105 trees / 7 loadouts**, one selector,
zero XML bytes and zero cached results. The 1,884 inline handles occupy **60,288 bytes**;
comparison document requests contain **6,213,222 XML bytes**. These figures exclude shared
data, import/catalog storage, allocator metadata and other ownership overhead; they are
not process peak memory or directly comparable total memory footprints.

Whole CLI process timing includes catalog/admission work, calculation, scoring, fresh
verification, serialization and export. The identical 2,940-state diagnostic problem uses
1,886 calculations and returns **19.02301632 DPS**, with the same Monk3/10364/24475,
quality-20 Wooden Club, Brutality I + Rapid Attacks I winner as the previous checkpoint.
Three-repeat wall-time distributions are **minimum / median / maximum milliseconds**:

| Workers | Typed candidates | Complete documents |
| --- | ---: | ---: |
| 1 | 163.4 / 164.5 / 313.6 | 390.4 / 397.0 / 434.4 |
| 2 | 166.3 / 167.4 / 169.8 | 312.2 / 315.3 / 322.7 |
| 4 | 160.4 / 161.6 / 171.8 | 252.8 / 259.8 / 278.7 |
| 32 | 171.9 / 175.6 / 175.9 | 222.5 / 227.7 / 231.9 |

The one-worker median whole-search improvement is about **2.4×**. More workers do not
materially improve the typed whole-search time for this small domain: setup, admission,
scoring and reporting now dominate. No inference about large-build or 5–30 minute search
quality follows from the numerical rate. Whole-search raw reports and script evidence
are in `runs/typed-release-check/` and `runs/verify-typed-release.py`.

Artifact SHA-256 identities:

- Native-only release CLI: `144fbddce25e5051f9fc95bd68629f2a30fa470dec72a22b707b10dc30e69a65`.
- Benchmark executable: `d95fd578a72a874bf5d20ff1b98b2c6c8b347b5d9738fd3f35405ff5577af224`.
- Unchanged data package: `b792b5c079dc7659cc588326a06c308a79f04464e7e023f703310662e02f2561`.
- Injected custom package: `c11cd406a78fa70431633bebe3c42bd18a87de32f6f48e92cd4e6bb2bebee91d`.

### Next implementation slice: local weapon-modifier assembly

The source audit selects a reusable prepared weapon-stat pipeline, then integrates it
with supplied normal/rare one-hand Maces on the two currently reviewed bases. Admit five
bounded effect families first: flat physical, flat fire, local physical increase, local
attack-speed increase and local critical-chance increase. This advances the agreed
broader equipment pipeline and introduces equipment/support interactions without a new
product-scope decision.

1. Add an injected item-rule model with stable identities, operations and local flags;
   keep base stats and the source critical-chance cap in data. Concrete modifier rolls
   come from exact supplied item text. Rust owns parsing/operation semantics, rather than
   hard-coded balance tables. Regenerate/review package and source policy versions while
   preserving the current tree/source pin and all independent goldens.
2. Share strict item admission between import and native profile parsing. Preserve raw
   payload/provenance, normal/rare rarity, base, quality, item level and explicit `LevelReq`.
   An item's equip level is distinct from item level. Do not claim affix-tier, acquisition
   or roll legality merely because a supplied modifier is understood. Reject unknown or
   unconsumed lines, conditional/global modifiers, alternate quality, sockets/runes/enchants,
   granted skills, uniques and unsupported damage families.
3. Translate actual local consumption and assembly into `PreparedWeaponStats` (name
   provisional), with legacy Mace entry points delegating to it. Source `Item.lua`
   `calcLocal` near lines 2384–2420 uses exact flags, zero keyword flags and the literal
   first-tag predicate `not mod[1] or mod[1].type == "InSlot"`; ordinary ModDB subset
   matching is not equivalent. Preserve consumed-versus-leftover evidence.
4. Follow `BuildModListForSlotNum` near lines 2462–2501: local rate rounded to two decimals
   before global/support speed; physical endpoints combine base plus flat, physical INC,
   then separate quality multiplication and integer rounding; fire does not gain quality;
   emit damage only when both endpoints are positive. Round local critical chance to two
   decimals, then apply the source cap before the second accuracy roll. Extract the cap
   from `CalcSetup.lua` rather than assuming local critical chance stays below 100.
   This also closes existing custom-base edge gaps in zero-endpoint suppression and local
   attack-rate rounding, currently outside the demonstrated native parity matrix.
5. Use actual `ModParser.lua` and `Item.lua` functions for interpreted/warmed oracle cases:
   local damage versus "to attacks", exact flags/keywords/tags, duplicate lines/removal,
   rounding boundaries, zero endpoints, quality, rate and critical cap. Add fresh complete
   PoB rare-item cases and intermediate weapon-stat evidence, with XML export/reimport.
6. Prepare each selected weapon once; typed handles continue to compose shared axes.
   Compare every legal bounded item/tree/support candidate against document evaluation,
   all seven loadouts and Monk speed with armour/fire encounters, custom injected data,
   explicit equip-level requirements, serial/Rayon archives and fresh verification.
   Retain zero-allocation numerical checks and measure preparation separately.

Source landmarks are for the pinned revision and must be checked during implementation.
General equipment, supporting actors/minions, complete native mechanic coverage and
realistic search-quality benchmarks remain separate unfinished work.

## Configurable support loadouts - 2026-09-08

Starting point: clean `ac8b441`. The prior resistance implementation passes both hosted
Windows/Linux jobs. This implements the source-audited support plan below, preserving
source revision, retained tree bytes and independent goldens.

| Checkpoint | State |
| --- | --- |
| Data and migration | Package schema **4**, semantics **`poe2-native-profiles-v4`**. New `supports` section replaces `mace.brutality`, with explicit identities, family/color, L1Q0, requirements, complete source eligibility, scoped typed modifiers, damage flags and cost metadata. Source Mace types and zero mana cost are retained. Nonzero costs and old package schemas reject. **154,848 bytes**, SHA-256 `b792b5c079dc7659cc588326a06c308a79f04464e7e023f703310662e02f2561`. |
| Source extraction | Policy schema 2/identity v4. Actual `Data.lua.makeSkillMod` preserves scoped flags. Dense-array guards reject hidden/sparse source values, unconsumed mechanics and compound type expressions. Seven source-oracle tests include all 1,024 represented skill-type subsets for each support in interpreted/warmed Lua. Fifteen loader and five extractor tests pass. Two release extractor processes reproduce the reviewed package and identical evidence; tree schema 2/44 nodes remains byte-identical. |
| Native core | Immutable `PreparedMaceSupports` uses existing ModDB semantics at dataset compilation. The prepared calculation does no key parsing or allocation and checks originating compiled-instance binding in O(1). Legacy Boolean wrappers remain for compatibility; native preparation uses explicit support handles. Physical INC/MORE, combined speed multiplier rounding, critical/ordinary armour and explicit damage disabling follow source. Composed INC below -100% rejects; rounded-zero MORE produces finite zero speed/DPS with source parity. Mace profile ID is `poe2-mace-strike-support-loadouts-v4`. |
| Projection and search | Canonical zero/one/two-key loadouts; new constructors/resolvers coexist with old wrappers. Exact gem instances, source order and comments survive import/export/removal. Native Mace evidence media version 2 includes exact support records. Problem schema 4 uses `support_loadouts` and exact `locks.support_loadout`; report schema 5. Legacy problem/report layouts retain their old scope. At most 105 × 64 × 7 = **47,040** alternatives, still under the 256 MiB preparation-work guards. Canonical rules and per-color aggregate requirements precede evaluation. |
| Targeted parity | **61 engine tests** pass, including **9,422** new cold/warm source comparisons and original six goldens. **76 import/native tests** pass. Fresh full-build parity covers **48** cases plus **two baseline pairs and two reimports**, all seven loadouts, both weapons, armour/fire-resistance scenarios, Monk speed and all four resistance owners. Seven CLI loadout tests cover serial/Rayon, tiny guided/exhaustive domains, exact/empty locks, tiny proposals, partial/empty budgets, custom modifiers and export replay. |
| Independent review | Fixed reserved `none` identity collision, source constructor wiring, composed negative modifiers and sparse/hidden source arrays. Follow-up data, engine, import/native, CLI and documentation reviews found no remaining blockers. Legacy source-data tests were migrated to the new support section; original goldens stayed unchanged. |
| Integrated validation | **398 unique workspace tests pass**, with nine ignored child helpers exercised by parents. The full suite recorded two stale migration-test failures; both corrected targets pass their reruns, with no unresolved failure. Latest-per-target evidence: `runs/support-test-coverage.json`; raw logs: `runs/support-workspace-tests.log`, `runs/support-schema-regressions.log`. **51 native-only CLI tests**, workspace/native-only Clippy, formatting, dependency isolation and five WASM libraries pass. Logs: `runs/support-{native-only-tests,native-only-clippy,workspace-clippy,fmt,wasm}.log`; isolated native test target directory avoids overwriting the reference executable. |
| Release reproduction | Reviewed full **2,940-state** search admits **1,884**, rejects **1,056** requirements and spends **1,886** attempts. Serial/four-worker feasible/infeasible archives and fresh finalists match. Winner: `class/10/asc/Monk3/entrance/10364/ascendancy-passive/24475/wooden-q20/brutality_i+rapid_attacks_i`, **19.02301632 DPS**, **7% chaos resistance**. Custom numeric support/resistance data replays with matching identity/trust; infeasible and over-budget cases do not export. Evidence: `runs/support-release-check/summary.json`. |
| Artifact identities | Reference release executable `baf47ab23c18deda78ec71a6023aea78fb804c495a0ceffc5d0164a22ef9a204`; native-only executable `a5939c124e2c0d4e0d56ab85ba05efc9b268a1b08dd1dcf79479618efac8b943`; extraction evidence `6acd477bed98419ab78bb3e8b500eecb27e79d7561e091728e4fb15a5242a546`; custom package `c11cd406a78fa70431633bebe3c42bd18a87de32f6f48e92cd4e6bb2bebee91d`. |
| Static audit | Strict UTF-8, local file links and heading links pass across 31 Markdown files. Nine unrelated data sections, all old Mace fields and Brutality values/identities are unchanged. Dependency manifests, source pin/full snapshot/tree, supplied originals and six goldens are unchanged. Evidence: `runs/support-static-audit.json`. |
| Publication | Code **`4845efe8b2829fe3ca6815f1054a8bde850623a8`** is pushed to main. [Windows/Linux CI run 34188064859](https://github.com/Azaril/poe-optimizer/actions/runs/34188064859) is **successful on Windows and Linux**, verified during the typed-candidate checkpoint. Completed snapshots: `runs/support-main-ci-complete.json`, `runs/support-main-ci-jobs-complete.json`. |

See [support loadouts](support-loadouts.md) for the configuration/API migration and
[the example](../examples/mace-support-search.json). These are bounded diagnostic
integration results; no general build-optimizer quality or new throughput claim is made.

### Planned typed native candidate slice (implemented above)

The source/code audit identifies repeated XML and JSON work in
`src/mutation_search.rs::Evaluator::evaluate`: materialization, native profile parsing,
result attachments/export, and strict realization reparsing. The numerical Mace kernel
already accepts immutable prepared support data. The next slice uses the existing
`CandidateEvaluator<C>: Sync` seam to measure and remove this overhead while preserving
switchable backends and the eventual general native calculation boundary:

1. Expose a privately validated typed controlled-Mace scenario/candidate view from import,
   created by strict source admission and bound to template/configuration/data/catalog
   identity. Reuse class/tree ownership resolution and requirements; do not introduce a
   second weaker legality path or accept arbitrary numerical fields as validated builds.
2. Prepare immutable class/weapon/loadout components, scaling preparation with the sum of
   axis sizes. Compose candidate indices into native calculation inputs and selected metric
   measurements. Avoid an XML-bearing `PreparedEvaluation` for each Cartesian candidate.
   The native candidate loop should perform no XML/JSON parsing, export or diagnostics.
3. Retain full document evaluation for the initial baseline and fresh finalist, with exact
   realization/export checks and the shared calculation ledger. Consider a default
   `CandidateEvaluator::verify` hook only if needed to keep verification counted once.
   PoB remains an explicit reference backend; it never becomes a native fallback.
4. Keep eager alternative `xml_sha256` semantics unchanged in the first slice. Catalog
   preparation/hashing is a separate measurable cost. Lazy payload hashing requires an
   explicit report/identity version and equivalent boundary checks before adoption.
5. Differentially compare every legal finite candidate's typed measurements against the
   full native XML path, plus fresh selected PoB cases. Include custom datasets, invalid
   owners/requirements/supports, identity mismatches, exact locks, serial/Rayon archives,
   partial/empty budgets and fresh exports. Retain dependency isolation and WASM checks.
6. Measure release distributions/checksums for full XML evaluation, prepared evaluation
   including attachments, the pure kernel, the typed metric path and whole search at
   one/two/four/available-core worker counts. Rotate real mixed candidates with uncached
   calculations; record CPU, dataset/backend identities, setup time and storage/allocation
   evidence. Avoid machine-specific speed thresholds in CI and kernel-only scaling claims.

This follows the already agreed native-performance architecture; no product-scope answer
blocks it. General support/active-skill, equipment, minion and complete native pipeline
coverage, realistic mapping/bossing search-quality benchmarks and GUI work remain future
work, independent of any improvement to this restricted search path.

## Signed resistance modifiers - 2026-09-07

Starting point: clean `3493cfd`. This implements the prior signed-resistance plan retained
below. It adds a reusable native player-resistance calculation and four source-derived
one-point ascendancy passive choices; selected configuration supplies every effect value.

| Checkpoint | State |
| --- | --- |
| Data and migration | Package schema **3**, semantics **`poe2-native-profiles-v3`**. `passive_effects` replaces `entrance_effects`, with explicit nullable owner IDs and 20 records. Five typed resistance operations permit signed finite values; old operations keep nonnegative guards. New `defence.resistance_maximum_cap` is extracted from the source global cap. Schema-1/2 packages require regeneration/review, never silent defaults. |
| Source/bundle | Policy-selected Warrior3/14960, Druid2/61722, Monk3/24475 and Huntress3/17058 are retained with exact source ownership, direct-root connections and complete parsed effects. Bundle schema **2** retains **44** nodes / **4,870** excluded. Tree **142,209 bytes**, SHA-256 `9ace0fac74dfbca4c7893b2061b24561253c415788770de9d1505166945a6829`; package **153,014 bytes**, SHA-256 `7f5c1ed6e959984df095a2f87ef96fb450ccfda7ea1b41cfeeb65f5bdd4a7de4`. Full source snapshot and revision stay unchanged. Maintainer preparation and normal supervised extraction remain separate paths. |
| Native calculation | Spark/Mace share signed player BASE resistance aggregation, elemental-versus-chaos rules, truncation toward zero, explicit global/base caps and floor. `CharacterModifiers::checked_add` validates combined ordinary/ascendancy effects; compiled lookup includes class/owner/physical ID without allocation. Profile IDs advance to `class-passives-v3`; backend implementation identity includes the shared resistance source. Fractional custom limits and base caps above the global cap now follow source; reviewed defaults are unchanged. |
| Projection and search | Typed selections add optional `ascendancy_node_id`, with at most one ordinary and one owned ascendancy passive. Problem schema **3** requires explicit ordinary/ascendancy budgets 0/1; schemas 1/2 retain their scopes. New reports use schema **4**, native-tree evidence schema/media **2**, and expanded catalog fingerprint v4. Both allocation kinds retain physical/effective IDs and exact configured effects. All locks and point/requirement checks precede dispatch; 105 choices compose into at most 13,440 candidates under the existing 256 MiB preparation-work cap. |
| Targeted validation | 27 data tests, four extraction-unit tests, six independent data-oracle tests, fresh bundle reproduction, 56 engine tests and 68 import/native tests pass. Five new CLI tests plus 32 fresh Spark/Mace full builds, two reimports and a root-only baseline pass; paired removal cases retain source parity. New 840-state serial/Rayon search admits 576 candidates, rejects 264 requirements and spends 578 attempts. Guided small-domain, signed custom feasibility, locks, zero-budget and exact export checks pass. |
| Review | Independent review found and fixed missing global-cap handling and missing shared-kernel fingerprint inclusion. Follow-up data/extraction and CLI reviews found no remaining blockers. Unknown mechanics, wrong ownership, excess allocations, old schemas and altered realization evidence reject. |
| Integrated validation | **374 workspace tests pass, zero failures**, with nine ignored child helpers exercised by parents. **44 native-only CLI tests pass**. Workspace/native-only Clippy, formatting, dependency isolation and five portable WASM libraries pass. The original six goldens and 100-case previous PoB matrix still pass. Logs: `runs/resistance-{workspace-tests,workspace-clippy,native-only-tests,native-only-clippy,wasm,fmt,release}.log`; dependencies: `runs/resistance-native-dependencies.txt`. |
| Release reproduction | Two fresh release extractor processes produce identical reviewed package bytes and identical extraction evidence. Native-only release serial/four-worker feasible/infeasible archives and finalists match; both complete runs use 578 attempts. Winner `class/10/asc/Monk3/entrance/10364/ascendancy-passive/24475/wooden-q20/brutality_i`: **16.62515712 DPS**, **7% chaos resistance**. Injected -7.5 makes the chaos constraint infeasible; a locked diagnostic export recalculates to -7 with matching data/trust metadata. Locked calculation uses three attempts; explicit over-budget domain uses zero. Artifacts: `runs/resistance-release-check/summary.json`. |
| Artifact identities | Native-only release executable `33613951ed8dd271916cea136f9c5a224fe454923437d21aca3c70202934cdb4`; reference-feature executable `b93c393b6d016f8b6d494727426df419ee7d633788d65b6d64cbf24199d093f2`; extraction evidence `3889aa8ddf4f9a7a777276b90e33958f3376e8bc357a84d07e6010c7530ab901`; custom package `a17358a98e29bfd23462dc94f5c679c10bafa93c4aa92522137cac46ce29674d`. |
| Static audit | Strict UTF-8, all 237 local file links and 20 heading links pass across 30 tracked Markdown files. Seven numerical/data sections, old defence fields and all 16 original entrance records are unchanged. Source revision/full snapshot, dependency manifests, original exports and independent goldens are unchanged. Evidence: `runs/resistance-static-audit.json`. |
| Publication | Code **`c91fab1ace2a649cbbe22fc2d5ccd999c247c72f`** is pushed to main. Local implementation, release and documentation checks pass. [Windows/Linux CI run 34185229809](https://github.com/Azaril/poe-optimizer/actions/runs/34185229809) is **successful on Windows and Linux**, verified at the next checkpoint. Completed snapshots: `runs/resistance-main-ci-complete.json`, `runs/resistance-main-ci-jobs-complete.json`. |

The [resistance example](../examples/mace-resistance-search.json) maximizes Mace hit DPS
subject to chaos resistance >= 1%. With reviewed data this selects the admitted Monk3 chaos
passive. These finite profiles remain diagnostic; they do not cover general ascendancy
paths, equipment requirements involving attribute dependencies, multiple supports,
supporting active skills, minions or complete game mechanics.

### Source-audited support plan (implemented 2026-09-08)

The source audit recommends replacing Mace's fixed Brutality boolean with zero-to-two
support loadouts drawn from three reviewed families. This extends the user's priority of
coupled skill/support/equipment optimization while reusing the native modifier query model.
Keep the current skill/normal-weapon scope and source revision until each expansion passes:

| Support | Source-derived operation | Pinned source |
| --- | --- | --- |
| Brutality I | Existing physical MORE and elemental damage-disable semantics | Existing source-derived record |
| Heavy Swing | PhysicalDamage MORE +35 with Melee flag; Speed MORE -10 with Attack flag | `src/Data/Skills/sup_str.lua:4188–4227` |
| Rapid Attacks I | Speed INC +15 with Attack flag | `src/Data/Skills/sup_dex.lua:4613–4642`; `src/Data/SkillStatMap.lua:2048–2050` |

1. Introduce a source-derived support catalog with identity/family/color/level/quality,
   eligibility, typed modifier operations and explicit damage-disable effects. Store numeric
   values in injected records. Compile immutable loadouts through shared modifier semantics;
   do not add one Boolean field per gem. Plan the package/input schema migration explicitly.
2. Preserve source ordering: `CalcOffence.lua:2888,2975–2980` combines speed INC and MORE before
   two-decimal rounding; `CalcOffence.lua:181–223` multiplies physical MORE before rounded
   damage endpoints. Smithing Hammer fire damage and separate critical/ordinary armour
   mitigation create useful interactions. Keep zero-cost Mace scope and reject unimplemented
   cost/reservation/conditional mechanics.
3. Validate eligibility and duplicate-family handling from `CalcTools.lua:108–133` and
   `CalcSetup.lua:586–625`. Aggregate every enabled socketed support's color cost using the
   existing requirement seam (`CalcSetup.lua:2244–2284`); red/red versus red/green combinations
   can change class/item legality. Preserve required main skill and exact support/item/passive
   locks, source ordering, disable-effect removal and source-preserving multi-support export.
4. Compare all seven unordered zero/one/two-support loadouts with tiny exhaustive and guided
   serial/Rayon references. Execute original modifier/speed/damage branches cold/warm, then
   fresh PoB builds across both weapons, armour levels, Monk speed entrances and admitted
   resistance passives. Test partial/empty budgets, duplicate-family rejection, custom data,
   exact export reimports and fresh finalist verification. Keep independent goldens stable.

No product direction question blocks this bounded extension. Full supporting-active-skill,
item-modifier and minion coverage, prepared candidate hot paths, broader native defence and
realistic mapping/bossing optimizer benchmarks remain unfinished.

## Class/entrance catalog composition - 2026-09-07

Starting point: clean `ed128f5`. The next slice of the existing plan composes admitted
class/ascendancy/ordinary-entrance choices with the current Mace equipment/support catalog.
It introduces no new game formulas, data records, dependencies or upstream revision.

| Checkpoint | State |
| --- | --- |
| Portable resolution | `poe_optimizer_data::class_tree` owns typed selection/resolution and a partial candidate graph. All 93 admitted combinations preserve shared root ownership, physical allocations and class-specific effective entrance sources. Native XML admission uses the same resolver. No complete tree extraction is fabricated from the partial bundle. |
| Catalog/materialization | `ControlledMaceCatalog::with_tree_choices` composes ordered selections with weapon/support payloads, binding full selected data and all payloads in catalog identity. Exact Build/Spec attribute edits preserve untouched source bytes; legacy constructors retain fixed Warrior behavior. Requirements use resolved selected class attributes. Preparation is capped at 11,904 candidates and 256 MiB of estimated source-hashing work. |
| CLI and legality | Problem schema 2 requires explicit available ordinary points 0/1 and ascendancy points 0. Optional supplied selections and independent class/ascendancy/paid-node/item/support locks precede dispatch. Report schema 3 records ordered third-axis choices and canonical admission evidence, alongside requirements and the existing full ledger. Legacy problem/report schemas remain 1/2. |
| Realization | Both backends validate the requested selected class/tree state. Native checks exact XML plus physical/effective entrance and configured-effect diagnostics; PoB checks its normalized export against the requested selection and immutable source frame. Finalists require fresh calculation and exact reloadable exports. |
| Targeted evidence | Eight native CLI tests and a four-candidate expanded PoB CLI search pass. Full 744-state serial/Rayon checks admit 504 combinations and reject 240 strength failures; all-entrance budget-zero domains consume zero evaluations. Twelve fresh mixed materializations across all eight classes and two fresh override exports match PoB; native diagnostic tampering rejects. |
| Integrated validation | **353 workspace tests pass, zero failures**, with nine ignored child helpers exercised by parents. **40 native-only CLI tests pass**. Workspace/native-only Clippy, formatting, dependency isolation and five portable WASM libraries pass. The unchanged six numerical goldens and 100-case prior PoB matrix still pass. Logs: `runs/class-search-{workspace-tests,workspace-clippy,native-only-tests,native-only-clippy,fmt,wasm,release}.log`, `runs/class-search-native-dependencies.txt`. |
| Review/static audit | Independent catalog/CLI and source-preservation/identity reviews found no remaining blockers. Audit passes: 30 Markdown files, 234 local file links and 19 heading links; strict UTF-8, source pin, package/tree artifacts and independent/original fixtures remain unchanged. Evidence: `runs/class-search-static-audit.json`. |
| Release evidence | Native-only release build passes. Complete serial/four-worker archives and finalists match, each using 506 attempts. Reviewed winner: `class/6/asc/none/entrance/3936/smithing-q20/none`, **22.760867499999996 DPS**. Prior custom package with quadrupled Wooden Club endpoints selects `class/8/asc/none/entrance/56651/wooden-q20/brutality_i`, **71.44014424999999 DPS**. Fresh XML reevaluation and backend/data/trust metadata match. Locked Witch 4739 resolves effect 17306 in three attempts; its explicit over-budget variant uses zero. Artifacts: `runs/class-search-release-check/summary.json`; executable SHA-256 `215828d7944f34ccbbac33169284afaaf8edb457ed03e423f9fa98c0fd438409`. |
| Publication | Code **`2135ed0af5ddee5d74d7eb66551f13f14a02900e`** is pushed to main. All local code, release and documentation checks pass. [Windows/Linux CI run 34181992200](https://github.com/Azaril/poe-optimizer/actions/runs/34181992200) **passes both jobs**. Completion snapshots: `runs/class-search-main-ci-complete.json` and `runs/class-search-main-ci-jobs-complete.json`. This following update changes documentation only. |

The [expanded example](../examples/mace-class-search.json) combines 93 tree choices with
four weapons and two supports. Complete default-data search takes 506 calculations:
one template, 504 legal alternatives and one finalist. This is a tiny integration reference,
not a realistic optimizer-quality benchmark. Ascendancy selections allocate no ascendancy
passives; the ordinary tree scope remains zero or one entrance. General equipment
self-dependencies, supporting active skills, multiple support slots and full game coverage
remain unimplemented.

### Prior plan: signed resistance modifiers

The read-only pinned-source audit identifies unconditional signed player BASE resistances
as the next bounded expansion. This makes defensive constraints respond to passive choices
without introducing attribute/equipment dependencies. Keep broader native mechanics as the
end state; these four source records are a validation slice, not a hard-coded Rust database:

| Owned ascendancy / physical node | Pinned source stat | Direct root |
| --- | --- | --- |
| Warrior3 / 14960 | +8% Fire Resistance | 5852 |
| Druid2 / 61722 | +3% to all Elemental Resistances | 35535 |
| Monk3 / 24475 | +7% Chaos Resistance | 74 |
| Huntress3 / 17058 | -20% to all Elemental Resistances | 36365 |

Source evidence is in pinned `src/TreeData/0_5/tree.lua`. `src/Modules/ModParser.lua:289`
maps elemental resistance separately from chaos. `CalcDefence.lua:926–963` adds elemental
resistance only to elemental types, truncates the final sum toward zero before floor/cap,
and `CalcSetup.lua:847–849` applies the penalty only to fire/cold/lightning. Current Spark
and Mace code duplicate the quest/penalty calculation and fixed zero chaos value. Next:

1. Introduce a shared player-resistance kernel using injected limits and typed signed BASE
   contributions. Preserve existing outputs and goldens. Reject max-resistance, INC/MORE,
   OVERRIDE, actor/minion/totem/DoT, conversion and conditional mechanics until separately
   implemented. Negative values must be allowed only for the new operations; preserve
   existing nonnegative checks elsewhere.
2. Generalize entrance-only effect records to explicit owned passive-effect records through
   a deliberate data schema/semantics migration. Extend source extraction policy to reviewed
   records and reject parser remainder, unexpected flags/tags/actor scope or extra modifiers.
   Keep numeric values outside Rust and excluded-record evidence explicit. Preserve the PoB
   revision; expanding retained data requires regenerated package/tree evidence, not relaxing
   source identity checks.
3. Extend typed selections/partial graph/materialization to zero or one admitted ascendancy
   passive, retaining zero or one ordinary entrance. Require caller ascendancy budget 0/1;
   store paid physical IDs and check ownership, connectivity and exact locks. Used counts
   must never supply available points. Update result/schema contracts explicitly.
4. Add original-source cold/warm parser and defence-branch tests for signed/fractional values,
   floor/cap boundaries, elemental-versus-chaos, quest toggles and penalties. Extend the
   existing character/Mace source harnesses; do not generate expected values with Rust.
   Compare fresh Spark/Mace builds across the four nodes with ordinary entrances and support
   on/off, all existing metrics and export reimports; confirm live ascendancy count/ownership.
   Add tiny serial/Rayon search, point-zero rejection, wrong-owner and lock cases, plus custom
   data changing constraint feasibility without borrowing reviewed parity status.

This remains within the agreed native/data/finite-search direction and needs no product
answer. Broader equipment, support and supporting-skill catalogs remain later slices.
Profile XML/result costs before selecting a prepared native candidate representation.

## Dataset-bound controlled search - 2026-09-07

Starting point: clean `6176123`. This completes the bounded D4 plan retained below. The
native CLI loads selected data once, shares the exact immutable snapshot with catalog and
evaluator, and checks requirements before dispatch. No runtime Lua or subprocess work was
added to native search. The PoB backend remains an explicit optional reference.

| Checkpoint | State |
| --- | --- |
| Requirement data and migration | Package schema **2**, semantics **`poe2-native-profiles-v2`**. Explicit level/attribute records replace weapon `required_strength` and cover the represented level-one active/support gems. Support color and per-color aggregate costs are injected. Missing/old fields, invalid colors/levels/integers and nonzero individual support attribute requirements reject. Regenerate schema-1 packages from pinned source, then review/reapply custom edits. |
| Source reproduction | All ten sections regenerate from verified source. **141,502 bytes**, SHA-256 `854dca85abcd031905761d9533b7437ce28e40b5154c23c99655084fbb719507`. Source evidence now has **25 direct files**, adding `SkillsTab.lua`. Seven sections and all prior numerical/identity fields remain unchanged after accounting for the explicit requirement migration. Tree artifact, source pin, supplied originals and six independent goldens are unchanged. |
| Catalog and scenario | `ControlledMaceCatalog::with_data` retains the selected snapshot. Skill/support identities and XML, normal-base classification, quest selectors/defaults and requirement records consume selected data. Full `DataIdentity` binds catalog fingerprints; equal-content separate snapshots interoperate, while cross-dataset candidates, scenarios and results reject. Native realization checks full selected backend identity; PoB reference binding admits reviewed-default content only. |
| Legality and dispatch | Typed available/required values and violations distinguish equip/use level from item level. The per-attribute requirement is the maximum of individual sources and the support-color aggregate. All alternatives remain available diagnostically; the CLI checks the lock-admissible domain before the template attempt and rejects illegal proposals before calculation. `empty_legal_domain` reports reasons and zero evaluations even when proposal budget is smaller than the product. Guided search can escape an illegal first point. |
| CLI/report/export | `search-experimental --backend native --data ... [--data-sha256 ...]` shares one loaded snapshot with catalog/backend. Required skill IDs are derived from selected records. Search report schema **2** includes identity, trust, requirement rejection evidence and the existing ledger. Native export companions retain actual backend/data identity, structured trust, package hint and XML hash. Exact locks, fresh finalist checks and no-overwrite checks remain. |
| Independent requirement evidence | Five PoB source-oracle tests pass, including actual gem requirement functions, repeated-color support counts with hidden/disabled exclusions, and the full pinned attribute-requirement aggregation block in cold/warm Lua. Weapon strength 11 and one red cost 5 need 11; active requirement 13 raises it to 13; three red supports need 15. These source probes do not expand the native catalog beyond its one support slot. |
| Integrated validation | **333 workspace tests pass, zero failures**, with nine ignored child helpers exercised by parents. **32 native-only CLI tests pass**. Existing six numerical goldens and the 100-case fresh PoB matrix pass. Workspace/native-only Clippy, formatting, dependency isolation and five portable WASM libraries pass. Logs: `runs/dataset-search-{workspace-tests,workspace-clippy,native-only-tests,native-only-clippy,fmt,wasm,release}.log`, `runs/dataset-search-native-dependencies.txt`. |
| Release evidence | Two release extractor processes produce identical reviewed packages and evidence. Default eight-state winner: **`smithing-q20/none`, 20.1596255 DPS**. Quadrupled Wooden Club endpoints select **`wooden-q20/brutality_i`, 62.429807999999994 DPS**. Serial/four-worker archives and finalists match exactly; fresh export reevaluation matches DPS and backend/data identity. Each complete search uses ten attempts; the all-illegal eight-state case uses zero. Artifacts and `summary.json`: `runs/dataset-search-release-check/`. |
| Artifact identity | Custom package SHA-256 `8b35214bbcc9d639d59286dd3124860729d3b4cc78b99dd9563a4559c11e42d3`; release executable `48bfb84621f8794163b9b8649b2de5ab2dad4813cfa1a1996a07e4a05a3b8d2d`; extractor `90640baa6adf5b71903d282c84bd08d9aa1352239244e77e316aedbfca2328fd`; extraction-evidence file `d642eced202189c9dd3f99548cd3604d228ad3a905022798e9ee068fe983668d`. |
| Review/documentation | Independent source/data-binding and CLI reviews found no remaining blockers. Seven new import tests, seven new CLI tests and a native scenario-reuse test cover requirement boundaries, maximum semantics, XML escaping, selected quest defaults, identity mismatches, locks and budgets. Static audit confirms seven unchanged sections, preserved prior numeric fields, valid UTF-8 and all 232 local file links plus 19 heading links across 30 Markdown files; evidence: `runs/dataset-search-static-audit.json`. |
| Publication | Code `9388935` is pushed to main. Local code, release and documentation checks pass. [Windows/Linux CI run 34179412604](https://github.com/Azaril/poe-optimizer/actions/runs/34179412604) passes both jobs, including formatting, workspace/native-only lint and tests, dependency isolation and five portable WASM libraries. This following update changes documentation only. |

At that checkpoint the requirement seam was deliberately scoped to fixed Warrior attributes with no
paid passives or attribute-granting equipment. It does not resolve general equipment
self-dependencies, all-game legality, broader source compatibility, browser execution or
optimizer quality. Requirements affect search admission; diagnostic evaluation remains
separate and does not become a recommendation certificate.

### Prior plan: class/passive catalog composition

The audit found no reusable production class/tree XML materializer yet. The existing
`NativeTree::resolve` in `crates/poe-optimizer-native/src/tree.rs` evaluates the admitted
choices; `tests/native_passive_parity.rs::case/cases` builds parity inputs with fixture
replacements. Those test helpers are not a production materialization API.

- Reuse `GameDataSnapshot::tree()` and
  `BundledClassTree::{class, ascendancy, entrances, entrance}` from the selected snapshot.
  Introduce a portable typed selected-class/tree resolver shared with native admission and
  a bounded bundle-to-candidate projection/composition seam. `TreeProjection::new` currently
  requires a complete `AuthenticatedTreeSnapshot`; do not treat the partial bundle as a
  complete extraction or introduce PoB extraction into native search.
- Extend controlled materialization to typed Build/Spec attribute spans: canonical class ID,
  owned ascendancy index/internal ID and zero or one ordinary physical node. Preserve other
  source spans, shared-root ownership and independent class/ascendancy/node/item/skill locks.
  Candidate roots remain implicit; use physical allocation IDs and class-specific effective
  views. Witch `4739 -> 17306` and Huntress `56651 -> 39263` are effect-source overrides,
  not replacement allocation IDs.
- Derive available attributes from the explicitly selected class, rather than mutable profile
  defaults or evaluator diagnostics. All currently admitted roots are statless and entrance
  operations cannot modify strength/dexterity/intelligence. A future attribute opcode needs
  explicit semantic and requirement-resolution work. Apply the existing maximum requirement
  check to the selected class before dispatch.
- Require caller-supplied ordinary point budget 0 or 1 and ascendancy point budget 0.
  `CandidateDomain` already checks paid costs, connectivity and locks. Observed allocation
  counts establish used points, never available points. Preserve rejection of selected
  special nodes and unsupported mechanics.
- Replace Warrior-specific realization assumptions with checks against the complete candidate,
  including native-tree physical/effective node and configured-effect evidence. Keep encounter
  inputs, untouched source metadata and fresh finalist exports guarded. Extend CLI axes/locks,
  versioned domain identity and the explicit preflight cardinality bound; the current resolver
  has two axes and a hard cap of 128 combinations.

There are 31 valid class/ascendancy identity selections and three ordinary-node choices per
identity (none or either class-local entrance), giving 93 structural combinations before
weapons/supports. The supplied eight-state weapon/support example would yield 744 combinations.
Keep preparation bounded and partial-budget reporting accurate. Validate coupled class/item/
support effects with fresh PoB comparisons, exact locks and one/many-worker tiny exhaustive
references; do not infer a complete interaction matrix from the previous 100-build parity
suite (98 distinct cases plus two reimports).

No new numerical formula or product decision blocks this bounded composition slice.
General attribute/circular equipment requirements, broader modifier extraction and source
revision migration remain later gates.

## Pinned package source extraction — 2026-09-07

Starting point: `fbf37c4`. This delivers the prior D4 source-extraction plan retained below: generate
all ten current sections from verified source, retain separate conversion-policy/source
identity, and publish through a bounded offline worker. The committed package and independent
source/golden tests remain unchanged. Broader source compatibility and custom-data search
remain separate work.

| Checkpoint | State |
| --- | --- |
| Source extractor | Four extractor tests and the new verified-read test pass. All ten source-generated sections reproduce the exact reviewed package. Source selections live in explicit policy JSON; typed modifier conversion rejects unconsumed fields/flags/tags and ambiguous records. Source byte hashes, policy and extractor identity are validated in the parent. |
| Worker and host | Five worker tests pass (plus one ignored child helper exercised by parent tests): deadlines/cleanup, ordinary bounded artifacts, canonical envelopes and collisions. CLI checks the deadline after host validation/serialization and before publication. Two-file output is explicitly nontransactional. |
| Independent validation | Three new CLI tests pass: fresh-process whole-package/evidence reproduction, all ten sections, 24 actual source hashes, native loading against an independent Spark golden, output preservation and invalid source/deadline rejection. Four existing independent Lua source-oracle tests pass unchanged. |
| Integrated checks | **316 workspace tests pass, zero failures**, with nine ignored child helpers exercised by parents. **24 native-only CLI tests pass**. The unchanged six goldens and 100-case fresh PoB matrix pass. Workspace/native-only Clippy, formatting, native-only dependency isolation and five portable WASM libraries pass. Logs: `runs/game-data-extraction-{workspace-tests,workspace-clippy,native-only-tests,native-only-clippy,worker-tests,cli-tests,source-parity,wasm,fmt}.log` and `runs/game-data-extraction-native-dependencies.txt`. |
| Documentation and review | All 231 local file links and 19 heading links across 30 Markdown documents pass. Independent integration review found and resolved the host deadline gap; no remaining concrete source-authenticity or typed-conversion issue was found. |
| Release reproduction | Two release CLI processes generate the same 140,853 package bytes and identical evidence. Native evaluation using the exported data matches the prior native backend identity and all nine measurements; Witch/entrance DPS is **9.342857142857143**, with the exact 2,831-byte source XML export. Artifacts and summary: `runs/game-data-extraction-release-check/`. Release executable SHA-256 `c0aa062251ef2492cbc5a6956a3575ee28c4d93889920b891d7bec3b2ab8ac57`; package SHA-256 `cfc9f4d0d6251e4d04e6ac1809dbcdd5459033cd61fbe6c4694b8346198d42a7`. |
| Extraction identity | Extractor SHA-256 `5baa1cab64c6f2b92c2ed4941d84622dd43f2019533200c401a63b99f5a6a8bd`; policy SHA-256 `7d56fa4aceab97737fce29aaed2923a7b49a121fadcda8ac951df9321f77a3a3`; evidence-file SHA-256 `3dcc27f5667c99445c1ba7f835039bdb39070afc2767d1d25056841c7313d8e3`. All 24 direct source hashes are checked independently; tree/loader/spec evidence remains in `package.tree.source`. |
| Publication | Code `2bd56de` is pushed to main. [Windows/Linux CI run 34176865681](https://github.com/Azaril/poe-optimizer/actions/runs/34176865681) passes both jobs, including formatting, lint, full tests, native-only tests/dependencies and five portable WASM libraries. The following living-document update changes documentation only. |

### Prior D4 plan: dataset-bound controlled search

This plan is implemented by the newer checkpoint above; validation and remaining work are
recorded there. The agreed acceptance scope was:

This can proceed with the current source pin and structural-tree guard. Keep the existing
Warrior/no-ascendancy/no-paid-passive search profile, two normal weapon slots and optional
Brutality I. Broader source compatibility, cross-class recommendations and general equipment
self-dependencies remain separate work. No new product decision blocks this bounded slice.

- Add explicit requirement records to the package and source extractor, with an intentional
  schema/version migration. Weapon strength is already data; support attribute costs and
  active-skill requirement semantics need source-derived records and coverage. Item level
  and equip-level requirements are different fields.
- Add a snapshot-retaining catalog constructor (proposed `ControlledMaceCatalog::with_data`),
  preserving the reviewed default convenience API. Resolve skill/support identities, names,
  item classification, quest normalization and generated gem XML from selected records.
  Escape generated XML attributes correctly and preserve existing source spans/restrictions.
- Bind catalog fingerprints and baseline/result validation to the selected `DataIdentity`.
  Replace the default-SHA-only native guard with exact selected catalog/data/backend agreement;
  retain default-only PoB reference binding. Identical-content snapshots remain compatible.
- Add a closed-profile requirement-validation seam before search dispatch. Pinned PoB aggregates
  support sockets by color times a data-derived cost (currently 5), then uses the maximum of
  that aggregate and individual item/active-gem requirements for each attribute. Weapon
  strength 11 plus one red support requiring 5 needs 11 strength. Additive
  `CandidateBudgets.resource_costs` would encode the wrong rule. Source anchors are
  `CalcSetup.lua` support counts and `CalcPerform.lua` requirement aggregation.
- Load CLI data once, construct catalog/backend from the same snapshot, derive required skill
  IDs from the catalog, and retain actual data identity/trust in reports and export companions.
  Keep diagnostic native calculation distinct from search legality and preserve finalist checks.

Acceptance: unchanged reviewed-default candidates/exports/objectives/parity; custom data
changes rankings consistently; escaped IDs/names round-trip; requirement values below/at/above
boundaries and maximum semantics work; mismatched candidates/scenarios/results reject;
custom PoB selection rejects; one/many-worker searches preserve locks, budgets and fresh
finalist exports. An empty legal domain must explain requirement failures without dispatching
calculations. General attribute derivation and circular equipment requirements are out of scope.

## Injectable native data packages — 2026-09-07

Starting point: clean `e4c8995`. The implementation delivers D1–D3 for the existing admitted
profiles and the external evaluation/benchmark portion of D4. It preserves the supported
build scope while separating numeric data from Rust calculation semantics.

- Added core `DataIdentity`, optional full instance identity on `CalculationBackend`, and
  engine validation of that identity. Prepared values own their compiled data and reject
  incompatible reuse. Same-content distinct instances work; data identity is separate from
  implementation/source fingerprints and trust evidence.
- Added bounded, duplicate-aware JSON loading, typed package records, immutable snapshots,
  explicit reviewed/custom trust and section/content digests. Unknown fields discarded by
  nested source enums and integer-key aliases also reject. The authoring helper validates
  before writing and refuses existing output; it does not generate authoritative source data.
- `CompiledGameData` resolves numeric records and typed entrance effects once. Spark/Mace
  kernels borrow injected data; resource/accuracy/quest/crit values, item bases, monster tables,
  defence parameters and passive magnitudes no longer live in Rust constant tables. Existing
  convenience APIs load the same reviewed package. Operation semantics and admitted input
  bounds remain code. Tree topology/class attributes retain the exact source guard.
- `evaluate`, `metrics` and `benchmark-native` support `--data` and optional externally supplied
  `--data-sha256`. Selected package errors never fall back. Evaluation schema 3 retains data
  identity; saved assessment accepts legacy schema 2 and new schema 3. Benchmark schema 2
  includes structured trust and separate initialization timing. Native XML exports carry a
  `.data.json` companion, including controlled search exports.
- Controlled Mace CLI search and its public native catalog binding admit only the reviewed
  default package. This closes a library path that could otherwise mix a custom evaluator
  with pinned materialization/requirement rules. All current identity comparisons include
  the data field. Custom-data search is deferred until those rules consume the same snapshot.

Package: 140,853 bytes; SHA-256
`cfc9f4d0d6251e4d04e6ac1809dbcdd5459033cd61fbe6c4694b8346198d42a7`.
Game `poe2`, schema 1, semantics `poe2-native-profiles-v1`, source pin
`3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`. The existing tree artifact, source manifest,
original exports and six independent goldens are unchanged. Reproduction and custom-data
commands are in [native data packages](native-data.md).

| Validation | Evidence |
| --- | --- |
| Portable model/loading | 15 data tests pass (six original and nine new), all-target Clippy and data WASM pass. Authoring helper reproduces identical reviewed bytes and rejects output overwrite. |
| Numerical integration | 52 engine tests pass, including five new injected-data cases and existing source/golden suites; engine Clippy and WASM pass. Logs: `runs/injectable-engine-tests.log`, `runs/injectable-engine-clippy.log`, `runs/injectable-engine-wasm.log`. |
| Independent data source | Four new tests compare actual pinned Lua values/defaults, both 100-level monster tables, skill identities, class attributes and all 16 typed entrance effects, including warmed functions. Source tests and Clippy pass. |
| Isolation/contracts | Three native-only tests pass: changed Spark values, A/B/A and concurrent isolation, equal-content instances, cross-data prepared rejection and dishonest boxed-backend identity rejection. |
| CLI and default-only search | 21 targeted native-only CLI/benchmark/search tests pass, including four new external-package/export tests and custom-data catalog rejection. Log: `runs/injectable-cli-targeted-tests.log`. |
| Integrated checks | **303 workspace tests pass, zero failures**, with eight ignored child helpers exercised by parent tests; **24 native-only CLI tests pass**. This includes the unchanged six goldens and 100-case fresh PoB matrix. Workspace/native-only Clippy, formatting, native-only dependency isolation and five portable WASM library builds pass. Logs: `runs/injectable-workspace-tests.log`, `runs/injectable-native-only-tests.log`, `runs/injectable-integration-clippy.log`, `runs/injectable-native-only-clippy.log`, `runs/injectable-wasm.log`, `runs/injectable-dependencies.txt`. |
| Review corrections | Reserved quest-selector collisions, nested discarded fields and integer aliases reject; benchmark reports retain trust; public pinned catalogs reject custom data; native exports retain dataset companions. Independent integration review found no remaining cross-data ownership/identity issue. A separate evidence review recomputed all 18 benchmark reports and confirmed the recorded figures and scope. |
| Release evaluation | The same native-only executable loads the reviewed package and a custom package with doubled Spark endpoints. Witch/entrance DPS changes from **9.342857142857143** to **18.685714285714287**; both 2,831-byte XML exports match the input, and their companion metadata matches result identity/XML hash. Artifacts: `runs/injectable-release-{example,custom}.json`, `.xml`, `.xml.data.json`; custom data SHA-256 `99a0b5868e44e2373edf248918ffbb6a7861c19c4ae38a23b417533982ead634`; release executable SHA-256 `1bfe1970b418ec63e41553a4709167f8a390f3ef7ae409cf96a3bc5479030e2c`. |
| Documentation | All 226 local file links and 19 heading links across 29 Markdown documents pass; `git diff --check` passes. Original fixtures and the PoB submodule remain unchanged. |
| Throughput | All 4,500,000 evaluations in 18 runs completed, with identical finite-metric checksums, stable backend/data identity and zero failures or late results. Fixed-profile timing and scope are recorded below. |
| Publication | Code `1a44c13` is pushed to main; [Windows/Linux CI run 34173951380](https://github.com/Azaril/poe-optimizer/actions/runs/34173951380) passes both jobs, including formatting, lint, full tests, native-only tests/dependencies and five portable WASM libraries. The following living-document update changes documentation only. |

The newer checkpoints deliver pinned generation and data-bound controlled materialization.
Remaining D4 work is broader source compatibility/update orchestration and general catalogs. Do not remove the structural pin or accept arbitrary
new operation versions merely to make a new package load. Browser execution and general
native build coverage remain separate gates. New full-result diagnostics change benchmark
work; previous throughput measurements remain tied to their recorded commits/profiles.

### Prior D4 source-extraction plan

This planned slice is implemented in the newer checkpoint above. Its acceptance scope was:

Add optional `extract-game-data` at the current pin, generating all ten package sections
without using the bundled package as a template. Reuse source verification, authenticated
tree extraction/projection and the existing bounded offline-worker supervision pattern.
Add a package extractor and supervised host command in the optional PoB crate/CLI; keep
Lua outside native evaluation. Verify the exact normalized bytes subsequently executed,
restrict module/library access, bound resource use and refuse output collisions.

Keep source values separate from reviewed conversion policy (profile selection, quest
positions, absent optional fields, typed modifier conversion and deterministic ordering).
Record source/file hashes, extractor/policy identity and output digest as extraction evidence.
Typed effects must account for the complete parsed modifier structure and actor/flag scope;
reject unconsumed modifiers or ambiguous reward records. Preserve the independent source
oracle and full-build parity tests. Require complete-package/section reproduction, repeated
fresh-process determinism on Windows/Linux, changed-source rejection and bounded failures.

This slice needs no new product decision. Retain schema, source pin, mechanic coverage and
default-only search. Broader source compatibility and data-driven search requirements remain
subsequent D4 slices; neither follows merely from a deterministic exporter.

### Current fixed-profile throughput checkpoint

Measured code `1a44c13bf8b973731174be3c79fd58bcb05ef5a7`, native-only release executable
SHA-256 `1bfe1970b418ec63e41553a4709167f8a390f3ef7ae409cf96a3bc5479030e2c`.
Reviewed external package SHA-256
`cfc9f4d0d6251e4d04e6ac1809dbcdd5459033cd61fbe6c4694b8346198d42a7`. Input
`examples/native-witch-entrance.xml` is 2,831 bytes, SHA-256
`fcb6ad36f0991733fe9ed9ec9fada97ee4cebab4d68247c1a53a50aab54c63ed`.

Machine: AMD Ryzen 9 9950X3D, 16 physical cores / 32 logical processors; Windows 11 Pro
10.0.26200; Rust 1.93.0. Run start: 2026-09-08 00:38:18 UTC (September 7 local time).
Each cell below has three separate CLI runs of 250,000 evaluations, with a 60-second
per-run deadline. No local Cargo or PoB work ran concurrently; OS background load was
uncontrolled. Prepared mode parses the immutable profile once; document mode parses it
per evaluation. Both recompute and validate full typed results, including XML/diagnostic
construction, Rayon scheduling, shared accounting and checksum observation. Results are
not cached. Package validation and compilation occur once per CLI invocation, outside the
iteration timer; backend/data initialization had a median 11.61 ms (range 10.00–21.95 ms).

| Mode | Workers | Median evaluations/sec | Min–max evaluations/sec | Median ratio to one worker |
| --- | ---: | ---: | ---: | ---: |
| Prepared | 1 | 66,280 | 66,003–67,546 | 1.00× |
| Prepared | 4 | 188,956 | 186,891–193,387 | 2.85× |
| Prepared | 32 | 351,947 | 337,109–370,360 | 5.31× |
| Document | 1 | 20,377 | 20,325–21,133 | 1.00× |
| Document | 4 | 56,401 | 55,812–57,872 | 2.77× |
| Document | 32 | 124,531 | 112,168–124,755 | 6.11× |

All 4,500,000 attempts completed with zero failures, late results, nonfinite/unavailable
metrics or backend/data identity changes. All 18 runs share metric checksum
`3dbf33c2ddbdc3595304372a141f9cfc98df9368736a196b7087ff99982c14d1`.
Reports, machine identity and aggregation are in ignored
`runs/injectable-throughput/{prepared,document}-jobs{1,4,32}-r{1,2,3}.json`, `machine.json`
and `summary.json`. Reproduce a cell with the command below; vary `--mode` and `--jobs`,
and use a new output path for each repetition:

```powershell
cargo run --release --no-default-features --locked -- benchmark-native examples/native-witch-entrance.xml --data crates/poe-optimizer-data/data/game-data.json --mode prepared --jobs 32 --evaluations 250000 --timeout-seconds 60 --output runs/unique-benchmark.json
```

This is fixed-input full-API throughput for the restricted Spark fixture. It does not
measure a bare calculation kernel, optimizer quality, general build throughput, PoB
speedup, allocation cost or browser performance. Prepared 32-worker iteration windows
were only 0.675–0.742 seconds, so the range matters. Diagnostics/result shape differs from
older checkpoints; those commit-specific figures are not a controlled before/after test.
Shared immutable `Arc` ownership and isolation are tested; realistic-build memory and
hot-path profiling remain future work.

## Injectable game-data design — 2026-09-07

The user clarified that most game data must load from configuration through its own model
and injection seam. [The accepted direction](game-data-boundary.md) extends the existing
portable data crate, makes backend/prepared identity instance-specific, and shares immutable
compiled data across workers. Game values and content become data; Rust retains arithmetic,
rounding and operation semantics. Embedded defaults use the same loader as external packages.
Data-only changes using supported semantics must not require rebuilding the evaluator.

This historical design checkpoint was documentation-only on `2675ffb`. The audit below
records its starting state; the checklist is maintained as implementation proceeds, and
current behavior/validation is recorded in the newer checkpoint above.

### Audit of the current data coupling

| Area | Current implementation / migration work |
| --- | --- |
| Backend and prepared ownership | [Native backend](../crates/poe-optimizer-native/src/lib.rs) stores only a clock; prepared profiles have no dataset owner/identity. Introduce shared data ownership and reject incompatible prepared reuse. |
| Tree access and provenance | [Native tree](../crates/poe-optimizer-native/src/tree.rs) obtains global bundled data and keeps static record borrows. Replace with instance-resolved IDs/indices and retained immutable ownership; diagnostics use the selected data identity. |
| Package loading | [Bundle loader](../crates/poe-optimizer-data/src/bundled.rs) uses a compiled expected digest and global `OnceLock`; [source model](../crates/poe-optimizer-data/src/tree_data.rs) fixes source/tree selection. Add portable byte loading with explicit trust/compatibility inputs while preserving source evidence. |
| Numerical content | [Spark](../crates/poe-optimizer-engine/src/spark.rs) embeds skill/resource/quest/resistance values. [Mace](../crates/poe-optimizer-engine/src/mace.rs) embeds weapon definitions, accuracy/support values and monster tables. Move records and patch-dependent parameters into the package; retain pure calculation semantics. |
| Further parameters and effects | Audit base evasion and other balance coefficients in [character](../crates/poe-optimizer-engine/src/character.rs) and [defence](../crates/poe-optimizer-engine/src/defence.rs). Replace native tree's twelve English-line effect mappings with typed effect records and versioned operation validation. |
| Admission and defaults | [Native profile](../crates/poe-optimizer-native/src/profile.rs) and native reporting embed skill/item identities and quest/encounter defaults. Resolve them from data; keep operation support and structural restrictions in code. Configured content must not expand unsupported mechanics by declaration. |
| Identity consumers | Global `backend_identity()` feeds results, [benchmarking](../src/native_benchmark.rs), [native search binding](../src/mutation_search.rs) and [catalog checks](../src/catalog_search.rs). Migrate these together. Core's current result/backend ID check alone cannot distinguish two datasets under `native-poe2`. |

### Migration sequence and acceptance gates

- [x] **D1 — data model and loading (current-profile scope implemented).** Define manifest, lightweight core `DataIdentity`, typed
  records and bounded byte loader in `poe-optimizer-data`. Separate owned validated snapshot
  from serialized DTOs. Cover the existing tree, character, skill/support, item, quest,
  encounter and level-table data first. Preserve raw-source provenance, partial coverage,
  physical/effective IDs and order-sensitive modifier semantics. Validate duplicate IDs/keys,
  ranges/units, references, missing tables and schema/game identity. Supply trust through the
  host; package claims do not authenticate themselves. Keep dependencies one-way.
- [x] **D2 — instance injection and identity (implemented).** Add compiled data ownership to native backend
  construction and prepared evaluation, replacing global tree reads and static record
  lifetimes. Kernels consume borrowed resolved data; compilation checks executable mechanic
  capabilities. Migrate backend identities, result construction, diagnostics, benchmark/search
  factories and catalog checks together. Version affected persisted contracts/cache identities.
  Prove A/B/A isolation, same-data cross-instance reuse and different-data prepared rejection.
  Keep the existing exact source guard until an explicit equally strict compatibility contract
  is ready; do not weaken admission simply to make edited packages load.
- [x] **D3 — externalize the current native numeric data (implemented; structural tree remains pinned).** Move the audited Spark/Mace constants,
  weapons, monster tables, rewards/defaults and patch-dependent formula parameters out of Rust
  and into the reviewed package. Replace English stat matching and numerical magnitudes with
  typed effect records whose operations are implemented in Rust. Separate generic operations
  from supported-profile admission. Default kernels and native documents must use the injected
  values; compatibility wrappers may not become a second hard-coded source of truth. A
  controlled synthetic package must change a supported result without Rust changes and must
  not inherit the reviewed dataset's identity or PoB parity status.
- [ ] **D4 — host loading, pinned extraction and data-bound controlled search implemented; broader source updates remain.** Compose CLI default/external byte loading through
  one path, with explicit selection failures and no fallback. Carry effective data identity into
  run/checkpoint manifests, exports and search configuration. Deterministic optional PoB
  extraction now reproduces the current complete package with separate source/policy evidence. Embedded and
  external copies of identical data must yield identical semantic identity/results. New source
  releases using supported operations update data/compatibility evidence without recompiling
  a Rust allowlist. Keep arbitrary custom packages clearly distinct from reviewed parity data.
- [x] **D5 — current-profile parity, portability and throughput evidence recorded.** Preserve the six independent
  goldens and 100-case fresh PoB matrix for the reviewed default data. Exercise invalid-package
  and cross-data cache/catalog cases, one/many-worker isolation, native-only dependencies and
  five portable WASM libraries. The signed-resistance slice adds fresh complete-build parity
  while retaining earlier fixed-profile throughput evidence. Measure initialization separately and confirm repeated prepared
  calculation neither reloads nor hashes configuration. Record memory ownership and supported
  fixture/machine identities before judging throughput. Class/entrance materialization now composes these rules; preserve this evidence while
  broadening source-derived modifiers and supported native build pipelines.

After D1 establishes concrete interfaces, numeric record extraction and independent
contract/parity fixtures can proceed in parallel in separate files. Keep backend identity
integration under one owner. A tree-only injection checkpoint does not complete D3 or the
requirement to externalize most game data.

Design verification: reviewed against the current code and independently audited for
ownership, provenance, compatibility and concurrency gaps. All 221 local file links and
20 heading links across 28 Markdown documents pass, as does Git whitespace validation.
One pre-existing stale M1 heading link was repaired. This checkpoint changes documentation
only; code checks were not rerun. No product-scope answer blocks D1; use versioned JSON and
complete package replacement initially, and preserve the native/optional-reference architecture.

## Native class and ordinary-passive evaluation — 2026-09-07

Starting point: `5dd77eb` on `main`, clean checkout. This implements the planned portable
data consumer and native class/ordinary-entrance calculation phases. No end-state scope
was removed. The supplied minion build and unrestricted native evaluation remain unsupported.

- `poe-optimizer-data` owns portable snapshot types, source identity/content authentication,
  automatic source views and finite graph projections. PoB compatibility paths remain;
  extraction and process supervision stay in the optional adapter. Snapshot schema 2
  preserves raw class-array positions separately from canonical integer IDs. Old schema-1
  snapshots reject explicitly and must be regenerated.
- The authenticated 131,681-byte bundle retains 40 source nodes: 28 physical implicit roots
  and 12 physical ordinary entrances, yielding 16 class-specific entrance views. Its coverage
  explicitly excludes 4,874 of 4,914 source nodes. Generation proves all 31 class/ascendancy
  selections have no implicit-root effects and that ascendancy selection leaves the admitted
  ordinary entrance effects unchanged. Witch/Sorceress and Ranger/Huntress shared roots,
  Witch node 4739's source 17306, Huntress node 56651's source 39263, and shared Lich/Abyssal
  Lich roots remain distinct evidence. The bundle is source data, never cached numerical output.
- `engine::character::CharacterInput` carries class attributes and nine resolved numeric
  modifier fields into `spark::evaluate_with_character` and `mace::evaluate_with_character`.
  Existing `evaluate` wrappers retain their six unchanged goldens. Both pipelines now compute
  armour/evasion as well as prior resources/resistances/hit outputs. Skill speed and flagged
  damage increases follow source rounding and modifier semantics; minion damage has no actor
  in these supported zero-minion profiles. Public metric availability is unchanged.
- Native document selection follows the active skill independently of class. Admission
  verifies class/ascendancy identity fields and admits roots plus zero or one class-connected
  ordinary entrance. Duplicates, foreign roots, unsupported paid ascendancy allocations,
  other ordinary nodes, choices, overrides, weapon sets and unknown stat forms reject.
  Immutable prepared inputs still run entirely in Rust without Lua/process calls.
- A `native-tree` diagnostic records native source resolution, physical allocations,
  effective paid-node stats/provenance and bundle identity. It explicitly sets
  `point_budget_verified: false`. The separate PoB live-pointer passive observation field
  stays absent on native results. Exports preserve admitted input bytes apart from the
  established explicit-options/cache cleanup. [The Witch example](../examples/native-witch-entrance.xml)
  demonstrates an ascendancy identity plus a class-switched ordinary entrance.

| Validation | Current evidence |
| --- | --- |
| Portable data and extraction | **21 targeted tests pass:** six Lua-free bundle/authentication tests, one exact fresh-source bundle reproduction, seven extraction tests and seven projection tests. Targeted Clippy and data WASM compilation pass. |
| Numerical source parity | **31 modifier/source tests pass**, including interpreted/warmed full ModParser semantics for all 16 effective entrances and both pipelines across levels/mitigation/rounding boundaries. Both independent golden tests covering six untouched fixtures pass; engine Clippy and WASM compilation pass. |
| Native admission/exports | Four new contracts pass across 62 class/ascendancy documents, 32 entrance documents, coupled shared-root cases, legacy/canonical class IDs, byte-preserving exports and unsupported input cases. |
| Fresh full-build parity | **100 fresh PoB evaluations pass** in a targeted 128.11-second run with two workers: 98 primary class/entrance/coupled-selection cases plus two native-export reimports. Maximum observed absolute and scaled numeric delta: **0** across public metrics and extra raw-stat comparisons. Mace cases have positive enemy evasion and uncapped hit chance. Native-only builds compile the oracle target with zero tests. |
| Full integrated validation | **277 workspace tests pass, zero failures**, eight ignored child helpers exercised by parent tests; **16 native-only CLI tests pass**. Workspace/native-only Clippy with warnings denied, formatting, native dependency audit and five portable library WASM targets pass. Logs: `runs/native-class-final-workspace-tests.log`, `runs/native-class-final-native-only-tests.log`, `runs/native-class-final-clippy.log`, `runs/native-class-final-native-only-clippy.log`, `runs/native-class-final-wasm.log`, `runs/native-class-final-dependencies.txt`. |
| Data/calculation compatibility | Native preparation now explicitly requires the bundle and both numerical pipelines to agree on rules revision, tree version and tree source hash. Twenty native tests pass, including mismatched/missing source identity cases; native Clippy and WASM checks pass. The guard changes admission only, with no formula/result-shape changes. Logs: `runs/native-class-source-guard-tests.log`, `runs/native-class-source-guard-clippy.log`, `runs/native-class-source-guard-wasm.log`. The complete integrated validation above includes this guard. |
| Initial integrated correction | The first full run caught one obsolete rejection assertion for newly supported node 4739; it now uses an unsupported node. No calculator change was required. The complete rerun passes. Initial failure log: `runs/native-class-workspace-tests-initial.log`. |
| Native-only release example | `evaluate examples/native-witch-entrance.xml --backend native --raw --timeout-seconds 30`: Witch/Abyssal Lich, allocated physical IDs `[4739,23710,54447]`, effective entrance source 17306. Life **809**, mana **315**, selected average hit **6.54**, selected hit DPS **9.342857142857143**. XML export is byte-identical; source/export SHA-256 `fcb6ad36f0991733fe9ed9ec9fada97ee4cebab4d68247c1a53a50aab54c63ed`. The final `36fc552` native-only release reproduces these metrics and bytes. Artifacts: `runs/native-class-final-example.json` and `.xml`; release build log: `runs/native-class-final-release.log`. This example is source-derived demonstration data, not an independent numerical golden. |
| Documentation and source preservation | 195 local file links across tracked Markdown documents and Git whitespace checks pass. Supplied original exports, independent fixtures and the pinned submodule are unchanged. |
| Publication and hosted CI | Published to `origin/main` through `36fc552`. Initial code `fa5136b` passes both [Windows/Linux jobs in run 34169436926](https://github.com/Azaril/poe-optimizer/actions/runs/34169436926). Final source-guard code `36fc552` also passes both Windows/Linux jobs in [run 34170105192](https://github.com/Azaril/poe-optimizer/actions/runs/34170105192). |

Bundle SHA-256: `272c40b13109c999e4e28693a5c64dfe9625ed24c106c83e2a132794ac955411`.
Full schema-2 source snapshot SHA-256:
`68445629df3af8bb934aad91f5e6b457f8fe7e478b67e306493ee9a017d171f7`.
Regeneration/authentication procedures are in [tree data](tree-data.md); the byte digest
comes from the checked-in trusted artifact manifest, not the input's self-asserted labels.

At this code checkpoint the next work was class/passive materialization. The new data
injection migration now precedes that search gate in the resume steps above. Native point-budget/gear legality, broader modifier coverage and all-six-dimension
optimization remain unfinished. Previous throughput measurements below apply to their
explicitly named prior code and fixed Spark profile, not automatically to this expanded result path.

### Release throughput for the class/entrance example

Measured code: `fa5136b`; input: `examples/native-witch-entrance.xml` (2,831 bytes;
source hash above). AMD Ryzen 9 9950X3D, 16 physical cores / 32 logical processors,
Windows 11 Pro 10.0.26200 x64, Rust 1.93.0. Native-only default release build/target
settings; no concurrent local Cargo or parity workload. OS/background load was not controlled.

Three independent 250,000-evaluation runs per mode/thread count, cycling repetitions,
thread counts and modes, with a 60-second deadline per run. All **4,500,000** evaluations
completed, with zero failures/late discards, stable backend identity and identical finite
metric checksums. No calculation warmups or result cache. Preparation is timed separately.

| Threads | Prepared typed results/sec, median (min–max) | Document typed results/sec, median (min–max) |
| --- | --- | --- |
| 1 | 89,798 (88,465–90,069) | 27,116 (26,942–27,463) |
| 4 | 259,882 (252,937–260,103) | 83,496 (82,347–85,391) |
| 32 | 466,612 (422,521–488,600) | 189,033 (179,382–193,643) |

Both modes recompute the native calculation and construct/validate complete typed results,
including the new tree/source diagnostics, XML export, Rayon scheduling, accounting and
metric checksums. Prepared mode reuses validated source inputs; document mode reparses and
resolves them each time. These are tiny fixed-profile API observations, not pure-kernel
speed, arbitrary-build throughput, search quality, browser performance or a PoB speedup.
Some prepared runs remain below one second and show substantial variation. The prior
checkpoint used another profile/result shape; the two tables are not a controlled regression
comparison. The preparation/result cost reinforces the separate compiled-input/minimal-output
search work in the resume plan.

Reproduce with `benchmark-native examples/native-witch-entrance.xml --mode prepared|document
--jobs 1|4|32 --evaluations 250000 --timeout-seconds 60 --output <new-path>`, three runs each.
Ignored artifacts: `runs/native-class-throughput/{prepared,document}-jobs*-r*.json`,
`machine.json`, `summary.json`; driver: `runs/benchmark-native-class-checkpoint.ps1`.
Per-result finite-metric SHA-256:
`3dbf33c2ddbdc3595304372a141f9cfc98df9368736a196b7087ff99982c14d1`.
Measured `fa5136b` native-only release executable SHA-256:
`48b12d9a52994ef41c12df31f5bfd3841d061ad8be7be26d958fea2f188a3587`.

## Native build pipelines, worker-free search and passive parity — 2026-09-07

The preceding native agent work stopped at 23 numerical/modifier tests; it was not a build
evaluator. This checkpoint adds complete admitted native document-to-result paths and uses
them in the controlled optimizer. The end-state design now explicitly requires a full Rust
replacement and native-only production packaging; PoB remains loadable for reference tests.

- Native Spark covers the unallocated Sorceress, level-one Spark, supported quest/resistance
  inputs and nine finite resource/resistance/hit metrics. Native Mace covers the unallocated
  Warrior, Mace Strike, two normal weapon bases, quality 0–20, item/character level 1–100,
  optional Brutality I and a normal enemy with explicit armour and optional evasion. Mace
  computes eight finite public metrics; selected average hit stays unavailable because PoB
  stores it per hand. Neither profile claims EHP, maximum hits, ailments, minions or Full DPS.
- `NativeBackend` validates XML into immutable `PreparedEvaluation`, runs actual translated
  math and returns the common contract with input/metric/source identities and supported
  exports. The pure calculation path allocates no state and calls no OS runtime. Typed
  evaluation has cooperative clock checks; browser hosts inject a clock. Cached source
  outputs are ignored and stripped from exports, and explicit options are baked into XML.
- Bounded import, structural preflight and controlled materialization now live in portable
  `poe-optimizer-import`. PoB module paths re-export them for compatibility. Native evaluator
  and search do not depend on the PoB adapter. The developer CLI retains its default PoB
  feature; `--no-default-features` packages native evaluation/search without Lua or reference
  commands. `--backend native|pob` switches the implemented evaluation/search backends.
- Controlled native search uses the existing Rayon CPU execution policy and shared budgets,
  exact candidate locks and fresh finalist checks. Native realization verifies materialized
  source, class/skill/item state and external context; candidate-derived conditions can vary.
  Four original independent attack goldens still validate the quality-zero interactions.
  Eight-state exhaustive/guided runs agree on `smithing-q20/none` (20.1596255 selected hit DPS).
- `benchmark-native` measures repeated prepared typed evaluation or complete document parsing
  and evaluation using an explicit local Rayon pool. It counts attempts, failures, late
  results, preparation and checksum observations; there is no cached-result or calculation
  warmup. This is fixed-input API throughput, not raw math speed or search quality.
- Native modifier programs now handle ordered PerStat/StatThreshold scaling with explicit
  unsupported special GetStat/actor/recursive forms. The engine has 34 calculation tests plus nine shared XML-parser tests, including
  both unchanged Spark goldens, four unchanged Mace goldens and source-executed numerical
  grids in interpreted and warmed LuaJIT modes. Class table index versus internal ID, enemy
  level clamping at 85 and monster physical-reduction cap 75 are modeled distinctly.
- Live passive coverage records actual allocated physical nodes, count buckets, roots,
  switch-source identities and displayed stats. The 49-build matrix covers all 31 class/
  ascendancy identities, 16 entrances, the supplied build and a weapon-set allocation, with
  at most two fresh PoB workers. Core coverage validates those observations structurally.
  An authenticated tree projection admits selected ordinary Normal nodes and preserves
  shared ownership/graph edges; selected unsupported mechanics reject explicitly.

- Review fixed quadratic export cleanup with one ascending source-span copy, including a
  20,000-cached-row regression preserving a large UTF-8/CRLF Notes payload. A shared lexical
  gate now rejects XML forms the pinned parser silently reinterprets: numeric references,
  spaced attribute equals, quoted delimiters, split CDATA and a leading UTF-8 BOM. Native adds stricter attribute
  normalization checks; reference preflight preserves the supplied build's valid multiline
  quest strings. Lossless import remains unchanged. Nine XML compatibility tests, including actual-source probes, cover those
  parser differences, and both backend fingerprints include the new gate implementation.

These native profiles do not yet support the supplied complex minion build.

| Validation | Evidence |
| --- | --- |
| Full Windows workspace | `cargo test --workspace --all-targets --locked`: **260 passed, zero failures**, seven ignored child helpers exercised by parent tests. Log: `runs/native-checkpoint-workspace-tests.log`. |
| Native-only CLI | `cargo test -p poe-optimizer-cli --no-default-features --all-targets --locked`: **16 passed**, including default-native selection, absent PoB commands, both benchmark modes, native search and exports. Optional reference tests are feature-gated. |
| Formatting and lint | Workspace formatting, workspace all-target Clippy and native-only CLI all-target Clippy pass with `-D warnings`. |
| Portable compilation | Core, engine, import and native library targets compile for `wasm32-unknown-unknown`; this does not establish browser execution. |
| Runtime dependencies | Native-only normal dependency graph contains no PoB adapter, mlua, LuaJIT or UTF-8 native module. CI now enforces this graph check. Log: `runs/native-checkpoint-dependencies.txt`. |
| Reference parity | Two unchanged Spark and four unchanged Mace goldens, actual-source numerical grids, 16 fresh Spark/Mace PoB evaluations including native-export reimports, plus the 49-build passive matrix all pass. |
| Release search | Native-only release CLI, `search-experimental --backend native --problem examples/mace-search.json --jobs 4 --max-evaluations 10`: template + eight candidates + fresh finalist, winner `smithing-q20/none`, selected hit DPS **20.1596255**, XML export written. Observed search/calculation duration **2.0101 ms**, excluding final persistence; a tiny diagnostic domain, not an optimizer-quality benchmark. Artifacts: `runs/native-checkpoint-search.json`, `runs/native-checkpoint-best.xml`. |
| Publication and hosted CI | Published to `origin/main` through `115a902` with explicit user authorization. [Windows/Linux run 34166302194](https://github.com/Azaril/poe-optimizer/actions/runs/34166302194) passes both jobs: formatting, workspace/native-only lint and tests, native-only dependency checks and portable WASM compilation. Initial native code `a09c406` first ran in `34165834314`, where Rust 1.98 found `chunks_exact_to_as_chunks` in benchmark accounting; `115a902` uses `as_chunks::<8>()`, retaining the arithmetic and passing both six-test benchmark suites and workspace Clippy on Rust 1.93. The following resume update changes documentation only; 179 local file links and Git whitespace checks pass. |

### Release fixed-input native throughput

Measured code: `a09c406` (the subsequent checksum-iteration lint fix retains its arithmetic).
Machine: AMD Ryzen 9 9950X3D, 16 physical cores / 32 logical processors, Windows 11 Pro
10.0.26200 x64, Rust 1.93.0. Built with `cargo build -p poe-optimizer-cli --release
--no-default-features --locked`, default release profile and target settings. No concurrent
Cargo/parity workload ran during measurements. OS/background load was not controlled.

Each row/mode used three independent runs of 250,000 evaluations and a 60-second deadline,
cycling modes and worker counts. All **6,000,000** requested calculations completed with
zero failures or discarded late results, identical nine-metric checksums across all runs,
and no calculation warmups or result cache. The fixed source was
`tests/fixtures/calibration/spark-mapping.xml`; metrics are the supported native Spark
profile. Report preparation time separately from iterations.

| Threads | Prepared typed results/sec, median (min–max) | Document typed results/sec, median (min–max) |
| --- | --- | --- |
| 1 | 165,064 (159,166–167,229) | 32,802 (32,567–32,852) |
| 4 | 403,810 (391,700–422,923) | 95,812 (93,992–96,070) |
| 16 | 699,762 (617,413–864,572) | 166,516 (164,984–167,140) |
| 32 | 1,074,253 (973,278–1,102,594) | 215,034 (211,105–215,866) |

Prepared mode reuses parsed inputs and reconstructs/validates the complete typed result,
including XML export and diagnostics. Document mode also repeats parsing and engine request
validation. Both include Rayon scheduling, shared attempt accounting and metric checksums;
neither measures raw math alone. Some prepared runs last under one second and show material
variation, so these are initial fixed-input observations rather than precise scaling targets.
They do not measure arbitrary builds, native versus PoB speedup, browser performance or
search quality. The preparation gap motivates keeping compiled native inputs in future
search hot loops instead of reconstructing XML and full presentation output per candidate.

Reproduction: build as above, then `poe-optimizer benchmark-native
<fixture> --mode prepared|document --jobs <1|4|16|32> --evaluations 250000
--timeout-seconds 60 --output <new-json-path>`, three repetitions each. Ignored artifacts:
`runs/native-pipelines-throughput/{prepared,document}-jobs*-r*.json`, `machine.json`,
`summary.json`; driver: `runs/benchmark-native-checkpoint.ps1`. These are local evidence,
not repository fixtures. Per-result finite-metric SHA-256:
`913cc534e08a78f9bc894e9db238723f799c5fa4e8973a1fa4074e44c1b7db49`.

## M2 controlled mutations, tree data and native multipliers — 2026-09-07

This checkpoint replaces fixed-document membership with a separate structural mutation
profile for supplied normal weapons/supports and prepares real tree data for broader
joint integration. It does not change the agreed release scope or calculation boundary.

- `search-experimental --problem <json>` accepts a source template, exact weapon alternatives,
  support choices, independent locks, configurable scalar objective/constraints and proposal
  settings. Exhaustive and guided modes share candidate/scoring/evaluation APIs. The example
  has eight combinations. One template calculation establishes scenario drift evidence and
  one fresh finalist calculation is reserved; failures count and all attempts share the run
  duration. Output contains exact catalogs, constraints, provenance and verification.
- `ControlledMaceCatalog` validates a structural profile, patches only item/support source
  ranges and preserves all other template bytes. It supports Wooden Club/Smithing Hammer,
  item level 1–100, quality 0–20, no support/Brutality I, character/enemy levels 1–100 and
  bounded explicit encounters. It rejects unsupported mechanics before calculation.
  Candidate-derived conditions may change; external inputs and persisted source state stay
  guarded. Parameterized realization is runtime drift evidence, not an independent golden.
- The discrete proposer samples up to 128 axes without constructing huge products. It cycles
  mutation radii, changes multiple unlocked choices and samples full restarts. Fixed axes
  remain fixed. An empty random sample can retry until explicit bounds, so it cannot suppress
  a later coordinated move. Seven tests cover exhaustive reference agreement, parallel
  determinism, extreme cardinalities, locks, fixed domains and empty-sample regressions.
- `extract-tree` runs a distinct supervised offline worker with startup-inclusive deadlines,
  private bounded artifacts, kill/reap and source verification. Its owned snapshot preserves
  all 4,914 physical nodes, eight classes, 23 ascendancies, 5,187 usable undirected edges,
  shared owners, overrides, typed source tables and all 14 dangling pairs. Extraction is not
  allocation legality. Rust snapshot types are currently adapter-owned; no Lua handles enter
  the serialized interface. This work uses only the pinned local submodule.
- Native numeric multiplier programs preserve explicit multiplier layers, BASE/OVERRIDE
  precedence, parent grouping, ordered Multiplier/MultiplierThreshold/Limit evaluation,
  dynamic divisors, caps and existing condition gates. Seven additional differential tests
  bring native coverage to 23 tests. Unsupported recursive or actor-target forms fail
  explicitly; real-build extraction and a complete native backend remain future work.
- Independent reviews caught early random-sample stalls, omitted canonical lock evidence and
  late timer start; regression coverage or integrated domain checks now guard those paths.
  Full exported-frame checks account for nondeterministic Lua table serialization while
  preserving ordered gems and candidate-derived conditions.

| Validation | Evidence |
| --- | --- |
| Full Windows workspace | `cargo test --workspace --all-targets --locked`: **188 passed, zero failures**. Five ignored child helpers are invoked by their parent tests. Log: ignored `runs/m2-mutations-workspace-tests.log`. |
| Formatting and lint | Workspace formatting and `cargo clippy --workspace --all-targets --locked -- -D warnings` pass. |
| Portable boundary | `cargo check -p poe-optimizer-core -p poe-optimizer-engine --target wasm32-unknown-unknown --locked` passes. Host search and PoB extraction are outside this gate; this is not browser execution or a speed measurement. |
| Mutation parity and CLI | Seven adapter tests and four CLI tests pass, including unchanged independent Q0 Mace goldens, Q20/level/Pinnacle realization, source-frame drift, locks, finite/guided search, partial budgets, output collisions and tree CLI behavior. |
| Documented supplied search | Ignored `runs/m2-mutations-64290528.json` and `.xml`: jobs 2, eight combinations, ten total attempts, no failures, consistent fresh verification and source export. Winner `smithing-q20/none`, selected hit DPS `20.1596255`, debug total `15513.8403 ms`. This is a single observation, not a scaling benchmark or new independent numerical golden. Adapter fingerprint `3fbb6e59049912f9e0bd914fd84fd3d46ab5a44a555231f3c203b36ade1353b8`. |
| Tree extraction | Seven source-data tests and two supervisor tests pass. Ignored standalone `runs/tree-0_5-64290528.json` has canonical snapshot SHA-256 `390e699115ac358b0da57b312e7449db919da124822365b888b5ade1dc97c433`; extractor fingerprint `a1c740c1ba3f8af90b9f423e1a973de8e14cd2cd938e1df9f4faebc004dd177c`. Identity-claim checks alone do not authenticate edited snapshots. |
| Integrity and review | Submodule, original supplied exports and independent goldens are unchanged. Independent scoped code reviews, README/docs local file-target checks and `git diff --check` pass. |
| Publication and hosted CI | Published as `55df9d5` to `origin/main`. [Windows/Linux CI run 34159259405](https://github.com/Azaril/poe-optimizer/actions/runs/34159259405) passed both jobs, including formatting, Clippy, full workspace tests and portable core/native WASM compilation. The following resume-document update changes documentation only. |

## M2 candidate/search and calibration CLI checkpoint — 2026-09-07

This checkpoint connects canonical finite candidates to a replaceable search/evaluation
boundary and a real PoB integration harness. It does not narrow the first usable optimizer
scope to weapon/support selection.

- Core candidate/lock contracts retain classes, ascendancies, passives, equipment,
  support gems and supporting skills. Fourteen tests include simultaneous changes in all
  six dimensions while preserving two required skill definitions and two exact items;
  reverse-edge connectivity, owner/point accounting, multiplicity, independent locks,
  resources and explicit unsupported mechanics are covered. Shared physical class and
  ascendancy roots/paid nodes use validated owner sets, including pinned Witch/Sorceress
  and Lich/Abyssal Lich relationships; effective override extraction remains explicit work.
- The host search crate uses Rayon for Rust CPU evaluations and scoped supervisor threads
  for external processes. Twenty tests compare a coupled six-bit search with all 64
  states, compare one/multiple jobs and both schedulers, preserve infeasible exploration,
  and exercise deduplication, failures, exact budget boundaries, cooperative cancellation,
  late results and fresh-verification drift. No Lua/process type enters candidate/scoring
  contracts. Search proposals remain domain-provided complete states.
- The PoB registry resolves only four independently calibrated source hashes, preserves
  original XML bytes and hashes exact item/gem payloads. Candidate membership includes
  immutable source/catalog identity. Fresh realization checks cover class roots, passive
  allocations, equipment payloads, gem identities/configuration and encounter settings;
  only verified upstream normalizations are accepted. See [bridge scope](pob-candidates.md).
- `search-calibration` exposes configurable scalar goals/constraints, jobs, shared duration
  and attempt limits. Four exploration attempts plus one fresh finalist attempt exercise
  the complete catalog. JSON includes catalog, source labels/hashes, effective budgets,
  backend identity, assessments and verification; an exact source export requires a fresh
  verified feasible incumbent. All results retain diagnostic status. The four-case DPS
  winner is Smithing Hammer without Brutality; the support-equipped pair ranks Wooden Club
  higher, preserving the independently demonstrated interaction.
- Native `Condition`/`ActorCondition` evaluation preserves parent/skill/actor fallback,
  override and weapon exceptions plus inactive MORE precision and source error ordering.
  Five additional actual-upstream tests bring the native crate to 16 tests. Explicit
  resolved condition tables are supported; FLAG-derived conditions, multiplier/scaling
  tags, real-build extraction and full native evaluation remain unsupported.
- Review fixes include last-round proposal admission, cancellation precedence and controls,
  late/failure accounting, full fresh assessment/metadata comparison, malformed finite
  evidence rejection, scenario drift, hidden item XML children and shared physical-node
  ownership. No user design input
  was needed; package separation implements the existing portable-core boundary.

| Validation | Evidence |
| --- | --- |
| Full Windows workspace | `cargo test --workspace --all-targets --locked`: **154 passed, zero failures**. Both ignored helpers are explicitly invoked by their parent deadline/fresh-process tests. Log: ignored `runs/m2-search-workspace-tests.log`. |
| Formatting and lint | `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets --locked -- -D warnings` pass. |
| Portable boundary | `cargo check -p poe-optimizer-core -p poe-optimizer-engine --lib --target wasm32-unknown-unknown --locked` passes with the shared-root model and condition implementation. Search's host scheduler is deliberately outside this gate. |
| Standalone calibration search | Ignored `runs/m2-search-9b151354.json` and `.xml`: jobs 2, five attempts including one fresh verification, `mace-smithing`, selected hit DPS `18.208694`, verification consistent. Debug search elapsed `7151.7147 ms`; this single observation is not a scaling benchmark. Adapter fingerprint `b2c24db59723ee873659e5cd044ae43bb3d39e0b25605ec8e5f70969833adafb`. |
| Integrity and review | Original fixtures/goldens and submodule unchanged. README/docs local file links and `git diff --check` pass. Independent review findings are addressed and covered by regression tests. |
| Publication and hosted CI | Published as `8e902c2` to `origin/main`. [Windows/Linux CI run 34156176185](https://github.com/Azaril/poe-optimizer/actions/runs/34156176185) passed both jobs, including formatting, Clippy, full tests and portable core/native WASM compilation. The following resume-document update changes documentation only. |

## Current repository and capabilities

- Branch: `main`; remote `origin` is `https://github.com/Azaril/poe-optimizer.git`
  ([repository](https://github.com/Azaril/poe-optimizer)). The user authorized publishing the
  project and supplied builds. The top resume point and checkpoint tables record the latest
  publication and validation state; dated earlier records retain their own commit evidence.
- [Workspace](../Cargo.toml): Rust 2024 / minimum 1.93; portable contracts, calculation
  kernels, authenticated portable game data, native document adaptation and shared import/materialization; a host search library
  and root CLI; and optional PoB supervision and native UTF-8 reference packages.
  [Core contracts](../crates/poe-optimizer-core/src/evaluation.rs) separate `CalculationBackend`
  from `EvaluationEngine`. Typed documents/options/results/coverage and identities expose no
  Lua or process API. The synchronous engine rejects missing/duplicate/undeclared metrics,
  incorrect units/schema, invalid finite metadata and changed backend/request identity.
  Backends enforce their own deadline; native calls are cooperative, PoB calls supervised.
  `EvaluationResult::validate_recorded()` also validates saved evidence, including optional
  passive observations and unused numerical fields. It cannot authenticate edited sources,
  reinterpret units against a new catalog or certify complete mechanics and game legality.
- [CLI](../src/main.rs): `import`, `evaluate`, `metrics`, `assess`, `search-experimental`
  and `benchmark-native` work in native-only builds. The default developer feature also adds
  `search-calibration`, `extract-tree`, `extract-game-data` and hidden PoB workers. `--backend native|pob` selects
  evaluation and controlled search where the reference feature exists; native-only builds
  default to native and reject the unavailable PoB choice. Evaluation reports use schema 3;
  the private PoB protocol and benchmark reports use version 2. Controlled-search reports
  use schema 2 for legacy problems and schema 3 for expanded class/tree problems.
  `--options` supplies supported skill/encounter selection; repeatable `--metric` filters
  typed actor queries, and `--raw` retains backend-specific diagnostic attachments.
  `evaluate --objective` compiles goals against the selected catalog and retains their
  required measurements. `assess` uses saved metric schemas/context without a calculation or
  today's backend catalog. JSON/XML destinations must be new paths. Planner conversion is
  not implemented.
- [Objective assessment](objective-assessment.md): `ObjectiveSpec` compiles into a
  backend-neutral `ScoringPolicy` maximizing/minimizing one typed metric with a conjunction
  of strict/inclusive constraints. It checks units, finite thresholds, positive violation
  scales, unique IDs and contradictory bounds. Primary value/score, constraint observations,
  shortfalls, normalized violations and strict-boundary flags remain explicit. Strict
  equality failure can have zero distance while remaining violated. Missing/nonfinite
  measurements make the assessment unavailable and are never rewarded as numeric scores.
- [Native backend](native-backend.md): `native-poe2` admits restricted Spark/Mace documents
  across all eight classes and 23 ascendancy identities with zero or one ordinary entrance.
  It computes translated pipelines and supports
  immutable preparation, typed evaluation, exact supported exports and host clocks. Native
  controlled Mace search uses Rayon directly and checks fresh realization without PoB.
  General native build mechanics and the supplied complex minion build remain unsupported.
- Metric catalogs are backend-specific. [Native](../crates/poe-optimizer-native/src/lib.rs)
  declares nine player queries: Spark returns nine finite resource/resistance/hit values;
  Mace returns eight finite values plus explicitly unavailable `selected_average_hit` because
  the contract does not aggregate attack hands. [PoB](../crates/poe-optimizer-pob/src/metrics.rs)
  declares 15 definitions and 17 actor queries, including diagnostic EHP and five maximum-hit
  values and selected-player/minion hit metrics. Units and availability are explicit.
  Combined DPS and Full DPS remain diagnostics rather than objective metrics. Unsupported
  actor/metric requests reject according to the selected backend's capabilities.
- [Options/context](../crates/poe-optimizer-core/src/options.rs) retain requested selection,
  external configuration and observed conditions. [Coverage](skill-coverage.md) records
  resolved/unresolved skill provenance, selected action and Full DPS membership/counts.
  [Live passive evidence](passive-coverage.md) adds actual class/ascendancy, allocated nodes,
  point-count buckets, shared roots and effective switch provenance in the PoB reference
  path. Observation is distinct from complete allocation legality or native tree semantics.
- [Importer](../crates/poe-optimizer-import/src/lib.rs): portable exact UTF-8 XML/hash
  preservation, 1 MiB share-code and 8 MiB XML bounds, complete zlib validation, 100,000-node
  parser cap and no DTD/custom entity expansion. Predefined/numeric XML escapes work.
  [Preflight](../crates/poe-optimizer-import/src/preflight.rs) rejects missing/ambiguous
  evaluation identity while preserving the separate container-only import contract.
  [Controlled materialization](../crates/poe-optimizer-import/src/controlled_mace.rs) owns
  finite Mace catalogs and exact source changes. Old PoB paths are compatibility re-exports.
- [PoB backend](../crates/poe-optimizer-pob/src/backend.rs) remains an optional supervised
  reference. Its [runtime](../crates/poe-optimizer-pob/src/runtime.rs) embeds `mlua 0.12.1`,
  LuaJIT `2.1.1787165859` and static `luautf8 0.1.6`; it does not load upstream Windows DLLs.
  [Native-module notes](../crates/poe-optimizer-lua-utf8/README.md) record exact provenance.
  The supervisor enforces startup-inclusive deadlines, bounded I/O, request identity,
  retained stderr and cleanup after kill/reap. Post-load guards reject stale calculations;
  source verification/fingerprints require no Git subprocess in a worker.
- [Independent calibration](calibration-reference.md): two Spark goldens retain 19 raw
  outputs each; four Mace goldens retain 25 top-level outputs and nine main-hand values.
  Mace weapon preference reverses with Brutality, demonstrating a real interaction. Separate
  C/Lua hosts used bundled LuaJIT `2.1.1784580905`; the expected values were not generated by
  the Rust evaluator. Native full-build tests now compare against those unchanged references,
  with fresh optional-PoB comparisons extending supported cases. This remains shared-source
  parity rather than an independent game model or legality certificate.
- [Native calculations](native-engine.md) include six numerical helpers, numeric modifier
  aggregation, Condition/ActorCondition contexts, multiplier programs and ordered PerStat/
  StatThreshold dependencies, plus complete admitted Spark/Mace numerical pipelines with shared
  class attributes, nine ordinary-entrance modifier fields and five signed resistance fields.
  Source-executed numerical and XML compatibility tests cover these slices and the shared import boundary. FLAG-derived conditions, broader tags,
  real modifier extraction and general build pipelines remain explicit gaps. The numerical
  engine has no production Lua/I/O/scheduling dependency; the native adapter adds portable
  parsing, contracts and injected timing. WASM compilation does not establish browser
  execution or speed. [Benchmarking](native-backend.md#fixed-input-throughput-benchmark)
  measures prepared or document typed API throughput, including result and accounting cost.
- [Tree extraction](tree-data.md) preserves pinned topology, shared roots, reverse-listed
  edges and automatic override provenance; 14 dangling connections stay explicit.
  [Authenticated projection](tree-projection.md) admits selected ordinary Normal nodes with
  owner sets and a graph containing only included endpoints. Unsupported selected mechanics
  reject. The portable data crate owns these consumers and the authenticated class/entrance
  bundle; native calculations apply supported entrance effects and the shared typed resolver
  composes bounded class/ascendancy/ordinary and ascendancy passive choices into source-preserving finite search.
  Broader passive effects and general tree mutation remain incomplete.
- [Candidates](candidate-model.md) and [search](search-kernel.md) provide finite rules,
  all-dimension locks, bounded parallel evaluation, proposals and fresh verification.
  `search-calibration` retains four immutable PoB sources; the [experimental command](experimental-search.md)
  composes source-preserving Mace choices with native or reference evaluation. General joint
  mutation/legality, shared memory admission, recovery and GUI remain unfinished.
  [objective.toml](../examples/objective.toml) is future notation; current policy input is JSON.
- [CI](../.github/workflows/rust.yml) checks the full development workspace on Windows/Linux
  and portable compilation. Native-only tests/dependency checks ensure deployment excludes
  the optional reference stack. Actual local/hosted outcomes belong in the current checkpoint
  evidence, not inferred from workflow configuration or older successful commits.

## Delivery plan and gates

Milestone IDs are stable references for review notes. A milestone is complete only when its
acceptance evidence is recorded below. M1 and M2 may overlap after the evaluator interface
is concrete; real-build recommendations depend on both.

| Milestone | Status | Deliverable and acceptance gate |
| --- | --- | --- |
| M0: bootstrap and alignment | Complete | Rust scaffold, pinned submodule, end-state design, source/prior-art review, preserved source fixture, and this implementation record. Scaffold validation passes; this does not establish evaluator correctness. |
| M1: evaluator and fixtures | Shared boundary, native class/entrance Spark/Mace pipelines, portable data and optional reference hosting implemented; broader legal mutation/mechanic coverage outstanding | Bounded import, typed metrics, native evaluation with cooperative deadlines, optional supervised PoB parity, controlled mutations across all six dimensions, export/re-import checks and independent calculation-state evidence. Complete the checklist below. |
| M2: reusable core and synthetic joint search | Candidate/lock contracts, synthetic joint search, deterministic parallel evaluation and bounded verification implemented; events, richer operators and shared resource management outstanding | Candidate/domain/lock models, metric policies, coupled mutations and repair, Rayon/job APIs, shared budgets and events. Match exhaustive tiny domains, escape coordinated-change traps, preserve multiple locks, and verify deterministic one/many-worker results plus cancellation/dedup/accounting. |
| M3: first usable joint optimizer | Not started | Integrate all six dimensions with multicore native evaluation and optional PoB parity; ship preflight, evaluation/comparison, search, saved-result reranking, recovery, verified exports, and offline reports. Meet the end-to-end gates below. |
| M4: broader catalogs and upgrade workflows | Not started | Extend mechanic/equipment/skill coverage and conditional upgrade/bundle ranking with explicit inventory, cost, and comparison semantics. Retain parity and lock guarantees. |
| M5: richer objective policies | Not started | Unit-checked expressions, composite and ordered priorities, soft preferences, Pareto selection, and explicit robust aggregation. Test policy-specific selection and preserve hard constraints. |
| Desktop GUI | Deferred until CLI/report contracts stabilize | Choose frontend; Tauri is a candidate. Reuse core jobs, results and comparison models. Verify CLI/GUI parity, responsive cancellation and native evaluator packaging; package optional reference workers separately. Does not depend on finishing every M4/M5 feature. |
| Native Rust calculation replacement | Active; all class/ascendancy identities, ordinary entrances and four resistance ascendancy passives supported by restricted Spark/Mace pipelines; injected data and finite native search implemented | Class/entrance materialization and explicit finite search rules are implemented; retain reviewed source compatibility, then broaden passive/modifier extraction, actor/skill coverage and full offence/defence while preserving differential parity and strict admission. Typed mixed-candidate preparation and API/whole-search measurements are implemented for the bounded Mace catalog; realistic broader performance, optimizer quality and browser execution still need evidence. |
| PoE1 adapter | Later, separate track | Add a distinct versioned rules/data/evaluator adapter after PoE2 interfaces are proven; do not mix game identities or reuse PoE2 parity claims. |

Narrow passive/item/skill experiments are internal validation steps. The first usable release
must search classes, ascendancies, passives, equipment, support gems, and supporting active
skills together within explicit finite catalogs. It must support 1..N required skills and
1..N exact equipped item instances, independently of other locks.

### M1 checklist: native calculation and optional reference parity

- [x] Pin and inspect upstream loading, calculation, mutation, and export seams.
- [x] Decode the supplied PoB export without altering source bytes; record provenance,
      game/tree identity, and structural checks.
- [x] **M1.1 PoB reference runtime — implemented and validated on Windows/Linux.**
      Embedded LuaJIT and static UTF-8 boot the pinned wrapper and load the supplied build.
      Host callbacks supply paths, time, logging, noninteractive failures and controlled
      scratch writes. Dynamic native loading is disabled; each VM stays in its worker.
      No pin change or syntax overlay was needed. Source-manifest reproducibility,
      mismatch rejection and native provenance checks pass, along with hosted Windows/Linux
      startup, native and fixture tests. Exact dependency/source identities are recorded.
- [ ] **M1.2 Boundary — implemented and locally validated; scenario coverage remains limited.** Shared
      `CalculationBackend` / `EvaluationEngine` separate application contracts from hosting.
      CLI schema 2 and protocol 2 carry explicit skill/scenario options, versioned identity,
      typed measurements, coverage and optional diagnostic attachments. Fake/boxed backend
      contracts, native profiles and reference option/coverage tests exercise this boundary.
      Part/stat-set overrides and a complete resolved scenario model remain unimplemented.
      The synchronous engine has no worker pool or preemptive native execution boundary.
- [ ] **M1.3 Import and metrics — container/preflight complete; semantics partial.**
      PoB typed units/availability cover 15 definitions and 17 actor queries; native profiles
      declare nine player queries, with Mace average hit explicitly unavailable. PoB coverage
      classifies the three unresolved supplied entries and exposes selected ownership,
      Full DPS membership, manual/generated provenance and missing tree connections.
      No automatic repair is performed. Extend selected minion command/usage-model coverage
      and expand action/mechanic coverage before treating measurements as search objectives.
- [ ] **M1.4 Baseline parity — partial, with independent calibration evidence.**
      Two self-cast Spark scenarios retain independently hosted/extracted same-pin PoB
      outputs and prior production parity. Four Mace/weapon/support references now add a
      ranking-reversal interaction. The native class/entrance matrix adds 100 fresh PoB comparisons,
      including coupled ascendancy/entrance selections and exports. Native and optional reference comparisons use these
      unchanged expected values; current validation is recorded in the checkpoint evidence. Per-hand attack AverageHit remains explicitly unavailable in
      the typed catalog. Reference generation uses different host, extractor and LuaJIT
      builds with shared calculations, so this is not independent game-mechanics certification.
      Expand to minion/usage and survival cases, more equipment/support interactions, and
      actual allocated paths. Same-adapter A/B/A/export comparisons remain useful separate
      evidence; cached export values are not a reference oracle.

- [ ] **M1.5 Mutation parity — partial class/entrance/weapon/support structural profile implemented.** Shared
      materialization preserves exact payloads and external settings for native and reference
      checks against the four Q0 goldens. Q20/level cases extend native admission evidence;
      Pinnacle realization is reference-only while native Mace admits normal enemies.
      Class/ordinary-entrance evaluation parity, reusable materialization and the bounded
      class/ascendancy/entrance finite search catalog are implemented; expanded validation
      and publication status is tracked in the current checkpoint.
      Exercise legal
      passive, item, support, supporting-skill, class and ascendancy changes separately
      and jointly. Preserve locks and verify export, point/resource accounting and effect
      removal against fresh materialization/reload. Include at least two required skills
      and two exact equipped-item locks; neither level-only changes nor the restricted Mace
      profile establish this gate.
- [ ] **M1.6 Isolation and failures — partial.** Parser/protocol failures, blocked-stdin
      timeout/reaping, scratch cleanup and fresh-worker A/B/A cover the reference path.
      Native prepared/document, serial/Rayon, timeout and export/reimport tests cover admitted
      profiles. Complete broader request-order and failure coverage. Persistent PoB reset/reload
      workers and memory-governed pools remain unimplemented; bounded fresh scheduling and
      cooperative library cancellation now exist. Enable reference worker reuse only after
      matching fresh-process results; native execution requires fresh independent calculation state.
- [ ] **M1.7 Measurements — harness implemented; broader benchmark program incomplete.**
      `benchmark-native` measures complete API calls in prepared/document modes with
      shared deadlines, bounded Rayon concurrency and explicit accounting. The mixed-candidate
      developer harness additionally separates document/result/kernel/typed-snapshot/owned
      measurement costs, with isolated whole-search comparisons. Current results
      belong in the top checkpoint evidence. Measure representative native and optional-PoB
      profiles separately, including preparation/result costs, memory and errors. Preserve
      enabled metric/profile declarations; do not require unimplemented EHP or confuse the
      historical standalone reference timings with a native scaling benchmark. Browser and
      realistic optimizer-quality measurements remain outstanding.

- [ ] **M1.8 Configurable native data — current-profile injection, extraction and controlled search implemented; broader updates pending.**
      D1–D3 provide packages, immutable injection, numeric data and typed effects. D4 supplies
      external loading, a bounded pinned source extractor and schema-2 data-bound controlled
      catalog/requirement rules and bounded class/entrance composition. Broader source
      compatibility and general catalogs remain.
      D5 current-profile evidence is recorded above.

The planner JSON export is preserved as auxiliary input/provenance. Raw PoB XML and PoB
share codes are runnable import paths; planner conversion follows only when its format
and missing semantics can be mapped faithfully.


### M2 and M3 acceptance details

M2 now has the initial objective/scoring boundary: one user-selected registered metric to
maximize/minimize and a conjunction of typed constraints, including strict/inclusive
comparisons. Unknown metrics, invalid units, contradictory bounds, nonfinite thresholds
and unsupported policies reject. Each constraint has an explicit positive violation scale.
The current API assesses supplied results and drives candidate ranking in the generic search
kernel and controlled CLI. It preserves metric schema evidence and keeps scoring
independent of game metric names; M5 extends policy implementations without redesigning
candidate search. Saved assessment is a new analysis of recorded values, not a new search.

M2 synthetic fixtures must include tiny exhaustively checked joint domains and a case where
every single-change improvement path stalls but a coordinated move wins. Exercise class
and ascendancy legality, tree connectivity/budgets, equipment multiplicity/slots, skill
compatibility/resources, and lock-preserving repair. Keep legal infeasible exploration
separate from verified feasible incumbents.

M3 must demonstrate:

- All six dimensions searchable in one real native-backed run with optional PoB parity, including cross-class
  alternatives. No silent freezing of a requested dimension or removal of a required lock.
- Complete-candidate verification and export/re-import checks independent of optimization
  cache/fast paths, with reserved evaluation/time budget and honest incomplete status.
- Shared CPU/memory limits across native Rayon tasks and any selected reference workers; bounded queues, in-flight
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
  Representative M3 throughput, quality and scaling results remain outstanding; the bounded
  Mace numerical/whole-search measurements above do not establish this gate.

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

### Historical objective, attack interaction and native modifier checkpoint — published and validated

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
| 2026-09-07 | `8e902c2` | Added canonical all-dimension candidates/locks with shared physical-node ownership, bounded host search and fresh verification, a four-build PoB calibration-search CLI and parity-tested native conditions. All 154 local tests, Clippy, formatting, WASM compilation, standalone search/export and review checks pass; hosted CI evidence is in the checkpoint table above. |
| 2026-09-07 | `55df9d5` | Added structural weapon/support mutation, supplied-problem CLI search, coordinated discrete proposals, isolated pinned-tree snapshots and native multiplier programs. All 188 local tests, Clippy, formatting, portable WASM and documented example checks pass; hosted CI evidence is in the checkpoint table above. |
| 2026-09-07 | `a09c406` + `115a902` | Added native Spark/Mace build pipelines, switchable backends, native-only packaging, Rayon controlled search, fixed-input throughput measurements and live passive/tree parity groundwork. All 260 workspace tests, 16 native-only CLI tests, formatting, lint, dependency checks and four portable WASM library builds pass locally; both hosted Windows/Linux jobs pass in run `34166302194` on main. Full native game coverage remains incomplete; class/passive calculations are next. |
| 2026-09-07 | `fa5136b` + `36fc552` | Added portable authenticated game data, native class/ascendancy identity and ordinary-entrance evaluation, source-alignment guards and fixed-input multicore measurements. All 277 workspace tests, 16 native-only CLI tests, formatting, lint, dependency checks and five portable WASM libraries pass. One hundred fresh PoB evaluations show zero observed numeric difference; both hosted Windows/Linux jobs pass in run `34170105192` on main. Next: source-preserving class/passive materialization, explicit search budgets and equipment requirements; full native game coverage remains incomplete. |
| 2026-09-07 | Injectable game-data design (after `2675ffb`) | Recorded the user-directed configuration/model seam, immutable compiled-data injection, instance/prepared identity, strict compatibility and D1–D5 migration gates. Reprioritized this work before broader class/passive search. Documentation only; runtime injection and external package selection remain unimplemented. |
| 2026-09-07 | `1a44c13` | Implemented current-profile data packages, immutable native injection, external CLI selection and data-bound identities/exports. All 303 workspace tests, 24 native-only CLI tests, lint, formatting, dependency isolation and five portable WASM libraries pass. All 4.5 million fixed-profile benchmark evaluations pass; both hosted Windows/Linux CI jobs pass in run `34173951380`. Source-update generation and data-driven materialization remain next. |
| 2026-09-07 | `2bd56de` | Added deterministic all-section pinned source extraction, separate policy/evidence, bounded offline supervision and the `extract-game-data` CLI. All 316 local workspace tests, 24 native-only CLI tests, lint, formatting, dependency isolation and portable WASM checks pass. Release regeneration reproduces the reviewed package/evidence and preserves native results; both hosted Windows/Linux CI jobs pass in run `34176865681`. Broader source compatibility and data-driven materialization remain next. |
| 2026-09-07 | `9388935` | Added schema-2 source-derived requirements, dataset-bound controlled catalog/native search, pre-dispatch legality and actual data/trust export evidence. All 333 workspace tests, 32 native-only CLI tests, lint, formatting, dependency isolation and five portable WASM libraries pass. Release custom-data ranking, serial/Rayon results and fresh export reevaluation agree; both hosted Windows/Linux CI jobs pass in `34179412604`. Next: production class/passive materialization and finite catalog composition. |
| 2026-09-07 | `2135ed0` | Added shared class/tree resolution and partial graph, source-preserving class/ascendancy/entrance composition, explicit caller budgets/locks, selected-class requirements and expanded native/reference search. All 353 workspace tests, 40 native-only CLI tests, lint, formatting, dependency isolation, portable WASM and release checks pass. Full 744-state serial/Rayon search admits 504 alternatives; fresh PoB interaction/export checks pass. Code is pushed to main; hosted run `34181992200` is in progress. Next: signed resistance modifiers and a small owned ascendancy-node slice. |
| 2026-09-07 | `c91fab1` | Added a shared signed-resistance kernel, injected global cap, schema-3 owned passive data/extraction and four one-point ascendancy passives. Problem/report schemas 3/4 preserve older input scopes. All 374 workspace tests, 44 native-only CLI tests, lint, formatting, dependency isolation, five portable WASM libraries, source reproduction and native release checks pass. New 840-state search admits 576 alternatives; complete builds and effect removal match fresh PoB. Code is pushed to main; hosted run `34185229809` is in progress. Next: configurable zero-to-two support loadouts. |

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

No product-scope answer blocks the next slice. Choose the project's distribution license before a public
release; confirm the desktop framework and packaging before GUI work; choose acquisition
data sources before trade/upgrade ingestion. Benchmark-specific metrics and usage profiles
must be documented when those fixtures are made runnable, without turning their choices
into mandatory goals for all users.
