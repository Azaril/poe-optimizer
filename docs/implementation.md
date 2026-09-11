# Implementation log and resume point

Last updated: 2026-09-11 (EDT)

**Current priority: real-build breadth.** The five supplied originals have **0/5 complete
native evaluations**; all five import/inspect and evaluate in pinned PoB. Shared parser
progress has not changed that result. Follow the [real-build rollout](real-build-rollout.md)
for the next integration gates, with Twister and Skeletal Sniper developed together and
all five originals exercising the input model. Crossbow remains an API/trigger stress
case: its saved reference selection has no hit-damage output. Unit/source test counts are supporting
evidence, not breadth completion.

**Accepted direction:** proceed with the [shared instance/resolution/plan migration](general-build-input-proposal.md).
The owner asks for the most correct structural design to avoid larger later refactors.
The [concrete API design](real-build-api-proposal.md) is the working contract; the B3
architecture question is answered. Prioritize source/instance/definition/plan separation,
provider lifetime and contrasting cases from all five originals. Ordinary implementation
choices need no repeated approval. Preserve numerical tests through explicit legacy adapters
without letting their single-profile assumptions define the new model.

**Current checkpoint: R2b authored configuration prefix paired with the original runtime.**
The [configuration stage](configuration-preparation.md) executes constructor defaults,
ordered authored inputs/placeholders, compatibility migrations and saved-set binding up to
the first activation continuation. It runs through ordinary native preparation and the CLI;
ready plans retain the source/view/data-owned token, and incomplete reports use schema 3.
Later configuration sections, control notifications, callback modifiers and prior root
setup remain explicit prerequisites. This does not complete effective configuration.

Schema 29 extends configuration to schema 2 with injected, source-authenticated loading
rules and labels. The package is 26,286,752 bytes, SHA-256
`8dfadca7567d7761cf8b01a9763bec8f2662abec45c500e3e271ee58edf8f9f6`.
All 28 other sections and all previous configuration metadata remain exact. A fresh normal
CLI export reproduces it. Constructor defaults reserve request budgets before cloning;
malformed local inputs preserve the original nonfatal diagnostics and later writes.

The independent oracle runs complete original startup, callbacks and calculations: eleven
source scenarios, ten paired native prefixes (all five originals plus five structural cases)
and a reused-runtime control pass. It covers duplicate keys, fallback, order holes, malformed
scalars, repeated/absent sections and nonfatal diagnostics. This is prefix parity; native
callback execution and numerical parity are not claimed. Source traces show 62 default-pass
callbacks before XML sections load, producing 52 player/21 enemy modifiers. Twister and
Sniper activate after Skills.Load and before their items/calcs/tree loads. Non-notifying
control placeholders are distinct from configuration placeholder writes.

Closed numerical adapters now require loaded explicit scalar and modifier-block projections
to match their raw source inputs before encounter overrides. This also guards fixed-scenario
candidate setup. Review caught the separate legacy customMods text path; a regression now
proves that injected text rewrites cannot bypass the adapter through either entry point.
Existing typed candidate calculations keep their zero-allocation/no-XML contract.

Native regression passed 112 tests; the run began before the final block-projection guard.
After that guard, six focused configuration/receiving tests and all nine view tests pass,
including unchanged numerical goldens. Eighteen configuration data tests, 23 package/skill
checks and five extraction tests pass. Strict workspace/native-only lint and five portable
library WASM checks pass. Seven default-feature CLI checks and fourteen native-only CLI
checks pass, including two fresh extraction comparisons and operation without PoB. The
integration ledger is `runs/r2b-integration-validation.json`; publication and exact-head CI
are recorded separately in `runs/r2b-publication.json`. The complete native supplied-build
denominator remains **0/5**.

**Resume point:** continue R2b with one shared typed-program owner/root interface and explicit
per-build writable state, then execute the initial default modifier pass and saved activation
for Twister and Skeletal Sniper together. Preserve parser API compatibility through an
adapter; do not create a fake parser catalog, second interpreter or per-setting Rust recipes.
Retain ordered captured closures, source-bound modifier/control methods, cumulative budgets,
separate UI/config placeholders and injected boss/monster definitions. The next useful gate
is actual ordered callback/modifier parity, followed by root/provider and actor/action
preparation. Keep all five originals and the unchanged R3/R5 whole-build denominator visible.

**Previous checkpoint: R2a authored skill loading validated locally.**
The [shared authored stage](authored-skill-preparation.md) runs through ordinary native
preparation and returns source/view/data-bound state. It processes all five original builds:
200 authored groups, 541 gem occurrences and 260 selected entries, including inactive saved
sets and source-order effects. Eight independent original-source parity tests pass. This
completes authored loading only; effective supports, configuration, items/passives,
providers, actors/actions and complete numerical evaluation remain unfinished.

Schema 28 adds injected preparation definitions for 966 gems, 1,436 effects and 22,004
numeric level rows. All 28 previous sections remain unchanged. Canonical runtime data keeps
unique/ambiguous lookup candidates; extraction evidence schema 2 retains observed Lua
traversal separately. Two fresh original constructions agree on the new runtime section.
No XML build fixture or frozen PoB result supplies production definitions or metrics.

`PreparedEvaluation::authored_skills()` retains executed state; `prepare-build` includes
it in both ready and incomplete outcomes (nested preparation report schema 2). Existing
Spark/Mace adapters reject processed states they cannot represent. Review also found and
fixed a candidate support-axis bypass: cold setup now validates introduced supports through
the same loader and retains per-axis failures. Valid repeated calculations retain no
source XML and preserve zero-allocation checks.

The native suite passed 100 tests before that final candidate fix; the two new regression
tests pass for both candidate APIs, and all 18 affected existing tests pass after the fix,
including zero-allocation and fresh-document comparisons. Twenty-nine CLI executions,
eight paired original-source tests, 36 focused data/exporter/loader tests and 20 import
regressions pass. Workspace/native-only strict lint, portable-library compilation and a
fresh complete CLI export pass. Exact-head hosted CI remains a separate publication check.
The full native supplied-build denominator remains **0/5**.

R1a owned instances, R1b independently selected views and R1c native entry points remain
in place. Six unchanged numerical goldens pass through owner/view and compatibility routes.
The R2b checkpoint above advances the next configuration dependency; continue effective
callbacks and root/provider lifecycle along Twister and Skeletal Sniper together.

The earlier G4 CI run finished with Windows search-budget failures and Linux implicit-line
mismatches; G3 succeeded on both platforms. The Windows failure is reproduced as preparation
contention under the tests' 30-second guard; test-only repair preserves all count assertions
and production deadlines. The Linux failure exposed a missing source range-pattern branch
for granted-skill implicits. Both [CI repairs](#g4-ci-repair-checkpoint) pass their local
regressions; fresh hosted Windows/Linux confirmation is pending. The earlier failed G4 run
remains failed. Evidence: `runs/r1a-prior-ci.json`.

| Breadth stage | Current state | Next evidence |
| --- | --- | --- |
| Source import/inspection | 5/5 supplied originals | Retain exact bytes, selections and unresolved entries |
| General native instance/actor/action model | R1a/R1b source model and R1c native boundary implemented; general effective producers pending | R1: source instances/selected views with explicit deferred producers; R2/R3: effective graphs |
| Authored native skill loading | 5/5 originals; 200 groups/541 entries, original-source paired | R2b: effective configuration, provider and actor/action stages |
| Authored native configuration prefix | 5/5 originals plus five structural prefix comparisons; activation pending | R2b: ordered callback effects and root lifecycle |
| Complete native real builds | 0/5 | R2/R3: paired Twister and Skeletal Sniper preparation and full outputs |
| Real-build mutation/parallel parity | Not run | R4: class/ascendancy/provider/support/item/tree changes and serial/Rayon agreement |
| Scenario breadth | Frozen originals cover Pinnacle/level82 only | R3: explicit mapping variants plus original bossing comparisons |
| Independent whole-build holdouts | Not yet established | R5: new missing-mechanism families; retain first-run failures |

The new [fixed diagnostic expectations](../tests/fixtures/breadth-expectations/README.md)
make the initial numerical denominator explicit: 22 existing public measurements per
original, 110 total. Exact selections/context and finite/infinite/unavailable outcomes
are preserved. The generic artifact checker runs no evaluator and cannot declare R3
completion. Mapping, full preparation, resource semantics and interaction/worker tests
remain gaps; the shared-model migration is now accepted.

Latest language work: **packaged native typed-program execution**, detailed in
the [language checkpoint](#typed-parser-language-decision-checkpoint). Schema27/parser7
contains 88 generated programs with four explicit permissions: three public Special
entries and their captured helper. The other 84 programs remain unadmitted. Public parser
parity and fresh supplied-corpus regressions pass within the documented scope; complete
native supplied builds remain **0/5**.

Earlier supporting implementation checkpoint: **injected common item-assembly policy**,
main commit `d428915`, from isolated code `5edf860c840232ae2940f7d3a51acf510aaa2ca1`.
Schema26 has 28 sections. `ItemAssemblyData`/`ItemAssemblyCatalog` is explicitly
**PolicyOnly**: it provides authenticated, injected policy for future native preparation,
not completed item assembly. All 27 prior sections remain exact; parser6 stays at
1,222 Pure recipes and 429 Unsupported dispositions. See
[the checkpoint](#injected-common-item-assembly-policy-checkpoint).

The frozen schema26 worktree passed 164 data, eight source/host, 69 native and three public
extraction tests, including two identical fresh exports. Native-only dependency isolation
and five portable-library WASM compilation checks pass. The combined main tree
passes 36 default/native-only CLI item tests and strict workspace/native-only lint,
recorded separately in `runs/common-assembly-integrated-validation.json`; the isolated
export binary retains its original identity. No fresh whole-build corpus is claimed.
Full native build admission remains Spark/Mace and complete supplied originals remain 0/5.

Main also contains the reusable native replacement-string checkpoint
`4f46f61b0b2906524930cb617305af9b1acfa518`, documented in `778ed4042400d38385137ba16cd463e37722fab5`.
Its 204 scoped tests and 84,713 original-source observations remain supporting evidence;
see [that checkpoint](#replacement-string-and-assembly-design-checkpoint). Its original
schema25 package and frozen `runs/common-item-assembly-worktree` are preserved.

The [native item-assembly design](native-item-assembly.md) remains supporting work. Runtime
assembly is a prerequisite to implement when reached by the real-build R2 paths, not a
new standalone family-port campaign. The schema26 supporting worktree is now frozen and
clean. No unimplemented assembly branch gains admission from its policy catalog.

When R2 identifies item assembly as a reached prerequisite, runtime steps include the
bounded identity-bearing preparation arena, exact local-query bit/keyword semantics, and
import-owned progress/provenance. Implement the entire
common `BuildModList` path through all reached slot calls and final cache clearing.
The 19 candidate items/43 slot calls identified from source predicates are validation
cases only. A collection prefix cannot report completed assembly. Add a lossless
original-source graph/lifecycle harness before admitting the completed common path.

Review inputs: `runs/common-assembly-data-api-review.md`,
`runs/common-assembly-oracle-api-review.md`, `runs/flag-factories-assembly-next-audit.md`
and `runs/number-factories-assembly-data-review.md`. Reuse existing VariantState counting;
handle injected alternate-count capacity explicitly. Nil-config queries need original
AND64/MatchKeywordFlags, including distinct live and captured mask dependencies.

The [conditional-parser decision](conditional-parser-operations-proposal.md) is now
**accepted: expand the typed rule language**. The owner has now also accepted the separate
**B3 actor/action/candidate migration**, prioritizing structural correctness. Follow
the [typed parser-program contract](typed-parser-programs.md) for authorized language work.
Preserve all frozen worktrees, evidence, 44 protected files and both caller inputs. The
breadth gates above supersede the older standalone-item continuation order.

The preceding number-conversion checkpoint is published as code
`8b679da344ce56519a605631ad9f0bb0ce7f255b` and docs
`b9902d0028d9b438f70e7a433f922ad222d75d03`, with 447 scoped test observations.
Publication/preservation records are complete in `runs/number-factories-publication.json`.
At 2026-09-10 00:31:56 UTC, its exact-head run34421312846 was linting on Windows and
testing on Ubuntu, with no failed steps; Flag/String runs were testing on both platforms.
The independent consolidated observation is `runs/common-assembly-prior-ci.json`.

The older String-checkpoint Linux trace-test failure has a published condition repair. The
[condition-trace checkpoint](#condition-trace-evidence-repair) keeps all required original
source consumers and rejects stale/aborted trace evidence. Fourteen scoped tests and
16 fresh repeat invocations pass; hosted Linux confirmation remains pending. This changes
only parity test infrastructure, not runtime calculations, package data or build admission.
The corresponding mixed-store observer repair is locally validated and independently
reviewed; its [checkpoint](#mixed-store-trace-evidence-repair) records the same scope limit.

The breadth review adds three acceptance gates: freeze required metric/availability
manifests before numerical implementation; prove a real calculation responds to changed
injected definitions; and compare reused worker transitions with fresh preparation. These
strengthen R3-R5 within the now-accepted shared-model architecture.

## G4 CI repair checkpoint

The GitHub REST observation confirms G3 run34552695729 succeeded on both platforms. G4
run34555917045 (`69db2cb`) failed its Test step on both: Windows class-search completion
assertions and Ubuntu three implicit granted-skill line-flag comparisons. Public job
annotations preserve the failures; full log download returned 403. Those historical outcomes
are not replaced by local results.

| Repair | Evidence and limits |
| --- | --- |
| Windows search completion guard | The unchanged native binary reproduces six pre-search `time_budget`/zero-evaluation outcomes with seven 30-second searches sharing two cores. A bounded 300-second test guard makes all seven finish 506 evaluations with equal verified results. All 8 native and 9 default class-search tests pass; every count, domain, legality, seed, worker and fresh-verification assertion remains. Production deadlines are unchanged. |
| Implicit granted-skill recognition | Native item loading now preserves the original exact-match short circuit, then substitutes positive integer ranges and performs anchored Lua-pattern matching against injected base text. It preserves pattern metacharacters, CR/NUL semantics, crafted/non-GAME guards, reached errors and cumulative work bounds, including empty spans. There are no item-name/fixture exceptions. |
| Final implicit regression | All 49 targeted tests pass: 2 unit, 37 import contracts, 3 new original-source tests and 7 existing provider tests. The unchanged 116-item corpus has zero mismatches across 702 parser/565 formatter prefixes, 106 complete preassembly states and 2 parser-frontier states. The broader provider matrix retains 1,597 paired cases and 1 explicit deferral out of 1,598, with zero mismatches. This is preassembly/parser coverage, not complete item or build evaluation. |
| Portability and review | Affected strict Clippy and workspace formatting pass; the final repaired import library compiles for wasm32-unknown-unknown. The full 396-test R1a baseline is separately bound to its pre-repair source. Reviewed data package and caller inputs remain exact. Hosted Linux/Windows validation must run on the published repair head. |

Ledgers: `runs/r1a-class-search-validation.json`, `runs/r1a-implicit-validation.json`,
`runs/r1a-implicit-source-freeze.json` and `runs/r1a-ci-repaired-import-wasm.log`.
Original failed/partial search reports and earlier boundedness/test-harness iterations
remain preserved. Final implicit machine source SHA is
`0ea9ea0be145684d51eef0a3e151ab8c78c908761cd1f1c502c1c59f509ba9f7`.
The reviewed package remains schema27/parser7 with SHA
`875155eb794d8356a0c79cfd8b26cbe36f46f6dc2c01c408a2a0aa5eff7758d3`.
Native complete supplied builds remain 0/5; resume R1b and retain the R2-R5 breadth gates.

## R1a source ownership and identity checkpoint

The new [owned import model](build-instances.md) preserves exact XML once and indexes every
element occurrence, including unknown content and independently failed typed projections.
It reuses existing skill/item consumer classifiers and bounded XML parsing. Stable typed
IDs distinguish saved sets, groups/entries, item records/slot uses and passive/config sets;
external source IDs remain raw data, with no guessed winner or default selection.

`inspect-build --with-instances` exposes a schema1 subreport and obtains lineages from the
host OS through CLI-only `getrandom` (the already-locked version0.4.3). Core/import have no
randomness, database or filesystem dependency. The existing decoder DTO remains compatible.
The identity allocator serializes a persistent watermark, preventing deleted-ID reuse when
properly restored; actual membership and coordinated candidate allocation remain separate.

Current verified scope: 10 core identity tests; 12 owned-import tests; five CLI tests with
each of native-only/default features. The five-source mapping retains 15 skill sets, 200
groups, 541 skill entries and 116 item records; this is structural evidence. Scoped core,
import and native CLI lint passes. Core and import compile for wasm32-unknown-unknown;
browser execution is not claimed. Broader regression passed all 396 tests across 36 target
results, with zero failures/ignored tests, on the pre-CI-repair source frozen in
`runs/r1a-root-regression-source.json`. Its original session56069 is terminal with exit0.

Supporting ledgers: `runs/r1a-build-identity-validation.json`,
`runs/shared-build-r1-cli-validation.json`, `runs/r1a-import-instance-tests.log` and
`runs/r1a-import-wasm.log`. Root source tests now also cover local namespace/reset contexts.
R1b preparation now includes ten independent original-source selector tests across 40 calls,
including 11 expected throws, sparse config IDs, independent passive positions, partial error
state and prior-spec jewel writeback. The full authenticated selection/load methods execute;
UI and non-selection preparation remain documented test boundaries. Source controls are not
a native resolver or full allocation parity. Evidence:
`runs/shared-build-selection-source-final.json`; the initial incorrect mixed-config default
expectation and its correction are preserved in the earlier run.

No complete native actor/action graph, selected-view resolver, editable candidate importer
or new whole-build result is claimed. R1b and R1c remain required before R1 can complete.

## R1b selected-source view checkpoint

Implemented portable `ViewRequest`, source/data-owner-bound `SelectedView`, separate
skill/item/config keys and passive positions, exact duplicate winners, explicit override
validation, generated/default origins, selected skill identity provenance and the optional
CLI report. Native `NumericSetKeys` preserves the original insert-only LuaJIT table layout
semantics needed by skill/config generated IDs. See [the contract](selected-views.md).

Review corrected costly missing-ID/spec scans, nested namespace diagnostics, text-budget
composition and override behavior for an invalid saved passive position. No definition
package, vendor pin or caller input changed. No database dependency was added.

Source/native pairing: 38 XML documents across four domains (152 observations), including
all five originals; eight source tests pass. This observes selection and registered-key
prefixes with explicitly inert item/skill/numerical producers. Fresh full PoB runs confirm
actual exported skill/item/spec/config selections: `1/1/1/1`, `6/6/6/1`, `1/1/1/1`,
`1/1/1/1`, `4/2/3/1`. Reference executable identity and complete results are recorded in
`runs/r1b-full-reference-selections/ledger.json`.

Evidence: `runs/r1b-selection-keys-validation.json`,
`runs/r1b-skills-config-validation.json`, `runs/r1b-shared-view-validation.json` and
`runs/r1b-integration-validation.json`. Earlier per-agent results are scoped to their recorded source hashes;
root integration results cover the final review corrections. Final local checks pass:
21 view tests, eight source-pair tests, four native key tests, three key-oracle tests,
six CLI tests in each feature configuration, strict affected-target Clippy, workspace
formatting and import/engine WASM compilation. The final CLI syntax-only cleanup has
an additional native-only selected-view test. Default CLI linking retains the earlier
LNK4098 LIBCMT warning; linking and all tests succeed. Hosted CI for the preceding
checkpoint remains in progress at the last observation; no hosted success is claimed.

Tests of this component do
not certify item registration, effective actors/actions, full LoadDB error ordering or
complete native supplied builds. R1c and R2/R3 remain required.

## R1c native preparation integration checkpoint

`NativeBackend::prepare` and ordinary `evaluate` now import, resolve and lower through the
shared owner/view path. Public `prepare_view`, detailed `prepare_request_with_lineage` and
portable `prepare_with_lineage` distinguish prepared plans from structured incomplete
reports. Existing Spark/Mace code consumes selected source nodes while preserving its
numerical and export admission. Explicit secondary weapon requests and mismatched owners
cannot silently use the original primary view.

Controlled build/Mace scenario preparation follows that same boundary, with portable
host-lineage variants. Native convenience import uses OS randomness outside calculation;
WASM callers supply lineage. Candidate calculations retain no XML or selection reports.
No new skill profile, definition package, fixture exception or database was introduced.

The library and new `prepare-build` CLI expose source-linked identity/producer diagnostics,
including the requested options and metric queries for override traceability.
For the five originals, selected identity entries total 260 and preparation issues total 493
(individual counts 62/46/62/62/28 and 111/94/106/115/67). These are source/preparation facts,
not active-effect counts or complete dependency closures. All five remain incomplete.

Validation ledgers: `runs/r1c-selected-profile-validation.json`,
`runs/r1c-preparation-report-validation.json`, `runs/r1c-native-view-validation.json`,
`runs/r1c-integration-validation.json` and `runs/r1c-native-regression-03.log`. New tests cover six
unchanged numerical goldens, exact ownership, caller changes/cache poisoning, explicit
view requests, injected catalog effects on diagnostics, resource bounds and concurrency.
Final full native regression is terminal success: 90 tests across 13 test targets, plus
an empty doc-test target, using four test threads to bound memory. The new preparation CLI
passes four tests under each feature configuration; ten existing native-only CLI tests
also pass. Strict workspace and native-only CLI Clippy, formatting, all five portable
library compilation checks, and native dependency isolation pass. The 191 local document
link targets checked are valid.

Regression caught and fixed an error-message compatibility change: detailed preparation
retains independent missing stages, while `into_ready` preserves the original adapter
error so full-document and typed candidate errors still agree. A new test initially
requested an unsupported metric and was corrected to an existing supported query; native
metric admission was not broadened. Windows CLI feature-switch validation was serialized
after a live executable blocked replacement. Earlier failed logs are preserved. The
reviewed data package, caller inputs and PoB pin remain exact. No fresh full-PoB numerical
run or additional whole-build parity is claimed. Hosted results for prior heads remain
pending in `runs/r1c-prior-ci.json`; publication state is recorded separately.

The next R2 slice is shared authored skill preparation for the original Twister and Sniper
paths: complete reached `SkillsTab.LoadSkill`/`ProcessSocketGroup` behavior and their
level/requirement helpers, using the existing full-data `skill_source_parity` harness.
Extend injected data with the missing preparation semantics (level rows, requirements,
flags and source construction winners) rather than adding another skill profile. Preserve
loading order across all saved groups and expose completed stage artifacts through the
public preparation path. Selected 46/28 entry comparisons are evidence of skill preparation,
not completed support application, actors or numerical output.

PoB's triggered-effect processing can clear a level cost through a shared definition
reference. Use a per-build overlay preserving alias relationships; never mutate the
immutable catalog shared by workers. Effective configuration, item/passive registration,
provider grants and actor/action construction remain subsequent reached dependencies.
`ItemAssemblyCatalog` is still PolicyOnly; actual registration needs completed assembly
and lifecycle source comparisons. Neither this boundary nor an incomplete report passes
any whole-build parity gate.

## Definition storage and UI search

On 2026-09-11 the owner raised DuckDB/ORM storage and pregenerated definitions, then asked
whether autocomplete could use data already loaded and questioned the database complexity.
The [storage assessment](definition-storage.md) recommends retaining generated packages and
adding a small derived in-memory UI search index. No database, ORM, new encoding or production
UI search is implemented. DuckDB is deferred; adopting it is not a required project phase.

- [x] **Storage assessment.** Audit current package/acquisition/compile boundaries and official
  DuckDB Rust/WASM/workload guidance. Definitions already ship as generated JSON; build XML is
  separate interchange. Record artifact-versus-logical identity migration requirements and
  conditional comparison criteria. No benchmark speedup or new numerical coverage is claimed.
- [ ] **UI definition discovery, at the CLI/GUI discovery milestone.** Expose a bounded query
  service over the selected snapshot, with stable definition references, display labels,
  filters, coverage and a derived prefix/substring index. Preserve duplicate names and data
  identity. Measure full-catalog latency/memory before adding fuzzy or specialized indexes.
  Validate old-response handling after data updates and context-dependent selection separately.
- **Conditional storage follow-up, deferred.** Reopen for measured load/memory bottlenecks,
  substantial catalog/patch-diff queries or authoring requirements. If justified, prototype
  DuckDB export to the existing canonical package, then compare JSON/compression, bulk database
  loading and portable binary encoding with equal fidelity/validation. Review evidence before
  adopting a format, migrating identities or adding a runtime dependency. These experiments
  are not prerequisites to completing the current native build evaluator.

At the storage-assessment checkpoint only documents changed. R1a implementation has since
advanced as recorded above; the database investigation remains deferred. Frozen game data,
imported builds and numerical coverage remain unchanged. Storage cannot substitute for the
missing actor/action producers needed by the five supplied builds.

## Seeded jewel opportunity: J1-J5 follow-up

The owner requested investigation of seeded Legion/Timeless items on 2026-09-10. Track
this as an optional joint-search extension, with the [design opportunity](seeded-jewel-search.md)
kept alongside the main design. A bounded source read found PoB lookup data and item/search
controls, but seed-based branches in the pinned PassiveSpec remain disabled pending data.
No seed generator, native provider, search or trade feature has been implemented. The
[J1 feasibility investigation](seeded-jewel-feasibility.md) now records exact source/data
incompatibilities, a bounded native reference-archive probe and missing corpus coverage.
Confirm game/version applicability and complete seed-data compatibility before implementation.
The G4 parser checkpoint remains separate; J2-J5 depend on the general provider/preparation model
and relevant R2-R4 real-build gates, rather than displacing them.

- [ ] **J1 — Source, applicability and cost investigation.** Verify actual supported item
  families by game/revision; trace seed/node/variant generation or lookup and existing PoB
  search. Source/distribution inventory and a one-family native decode probe are complete.
  Complete intended-game mapping/reference and provenance remain open; lookup/transform
  costs still need compatible data. Measure full evaluation/search costs during J2/J5 once
  those paths exist. Decide whether to reuse tables, port a generator or combine both.
- [ ] **J2 — Injected provider and transformation parity.** Model seed identity and passive
  changes through the shared data/provider seam. Prove complete node transformations and
  whole-build results, including radius, overlaps, exact item locks and removal/readdition.
- [ ] **J3 — Opt-in joint search.** Add bounded seed/socket/path proposals, lazy catalogs,
  revision-aware caches, parallel batches, cancellation and explicit search-domain reporting.
  Evaluate interactions with every unlocked build dimension using the user's objective.
- [ ] **J4 — Discovery and trade handoff.** Report reproducible exact item seeds, socket/tree
  changes and metrics; distinguish theoretical candidates from supplied/available items.
  Investigate searchable trade exports and a transformed-tree comparison visualization.
- [ ] **J5 — Breadth and usefulness gate.** Use independent families/seeds/sockets and full
  builds; prove serial/reused-worker/cache parity and compare search quality/cost with
  fixed-tree seed ranking and ordinary joint search under equal budgets.

### J1 source/data feasibility checkpoint

The previous goal turn made progress by publishing G4 code `69db2cb` and resume update
`d349468`. This turn investigated the owner's seeded-item opportunity without changing
production APIs, package data, the source pin or caller builds. The broader B3 model
question remains pending; source research does not authorize that migration.

| J1 subgate | Current evidence |
| --- | --- |
| Pinned PoB2 source path | Read-only audit records 29 excerpts/17 source hashes. Active radius/conquest and partial keystone/Tribute handling coexist with disabled seed-dependent branches and no initialized lookup globals. Legacy numeric family IDs mean different things in the unfinished PoB2 application. |
| Reference data availability | Sixteen bounded official metadata/source requests locate 11 PoB1 families totaling 169.27 MiB compressed. Newer PoB1 node maps, local/global IDs, variants and formats differ from the pinned PoB2 inputs. A matching display name/seed range proves no compatibility. |
| Single-archive native probe | One authenticated 2,181,337-byte PoB1 Heroic Tragedy zlib stream decodes to 3,587,054 bytes. Five fresh Rust decoder states match the independent checksum; median 22.39 ms decode, hashing separately 1.28 ms. A 1 KiB output-bound control rejects before oversized growth. No lookup, transformation, browser or full-build timing is claimed. |
| Corpus coverage | All 116 items were inspected: 18 jewels, three radius items, zero seeded items. Five empty TimelessData UI defaults are not mechanic coverage. Existing selected-view reference reports have 613 passive observations, none conquered; inactive trees are outside that count. |
| Remaining J1 evidence | Complete PoE2-compatible seed/node/variant reference, source-specific boundary behavior, dataset provenance/distribution terms and actual lookup/transformation costs remain unresolved. Project MIT and generated game-data copyright notices do not supply a per-pack provenance manifest. |
| Integration | J2 should investigate optional content-identified packs; the existing eager JSON package is 23,431,124 bytes with a default 32 MiB limit. No packaging API is changed. Full seeded build fixtures and J2-J5 remain future work; complete supplied native builds remain **0/5**. |

The [feasibility report](seeded-jewel-feasibility.md) records the source links, immutable
external revision, asset hashes, measurement limits and follow-up requirements. Local
ledgers are `runs/seeded-jewel-j1-source.json`, `runs/seeded-jewel-j1-data.json`,
`runs/seeded-jewel-j1-corpus.json`, `runs/seeded-jewel-j1-reference-coverage.json`,
`runs/seeded-jewel-j1-loading-seam.json` and
`runs/seeded-jewel-j1-probe/native-bench/validation.json`. Independent source and data
reviews found no factual blocker in the report; the corpus and decode probes retain their
separate scopes. No new complete-family or build admission is claimed.

G4 exact-code run34555917045 is still live on Windows/Linux with no observed failed step
in `runs/seeded-jewel-j1-code-ci-final.json`; this is a verified nonterminal observation.
Resume by handling any actual CI failure and the pending shared-model decision, then
R1-R5. J1's missing reference is recorded for future follow-up rather than replaced with
an invented PoE2 mapping or another isolated helper implementation.

## Typed parser-language decision checkpoint

On 2026-09-10 the owner selected **"Expand the typed rule language now"**. The accepted
[ADR](conditional-parser-operations-proposal.md) supersedes the focused Rust algorithm
recommendation. The new [program contract](typed-parser-programs.md) defines injected,
versioned control flow, lexical locals, explicit call/result packs, table ownership and
effects, source binding, diagnostics and bounded execution. Rust validates and lowers
programs to immutable plans; the initial executor interprets those plans in Rust. This
is native preparation infrastructure, with no Lua or subprocess in its execution path.
It does not prescribe a bytecode representation for every combat calculation.

The first complete source targets are GemProperty and grantedExtraSkill, translated
through reusable instructions rather than named runtime handlers. Their original behavior
requires lazy branches/lookups, mutable local-table aliases, helper-owned captures and
a distinction between zero return values and an explicit Nil result. Existing immutable
public tables cannot stand in for the mutable invocation heap. Source-defined string
substitution, property selection and requirement names belong in program data.

**G1 is implemented:** the standalone schema1 `ParserProgramCatalog` owns its injected
parser catalog and validates all branches, scopes, source references, bindings, result
packs and aggregate limits. Mutable parameters, raw byte literals, final-result expansion
and declared intrinsic identities are explicit. Table append carries its declared, structurally validated
TableInsert binding, distinct from literal list positions. Runtime ownership checks remain
necessary for dynamic writes; validation does not claim to solve general alias analysis.

`CompiledParserPrograms` prebinds calls and lowers structured control flow into immutable,
source-mapped instructions. Nested breaks, empty-loop backedges and raw fallthrough remain
explicit. It does not evaluate expressions during compilation or inline recursive helpers.
The compiled library retains its exact owning catalog and can be shared across threads.
Both data and adapter implementation fingerprints include the new Rust sources.

The G1 publication `ed660f6` contained no executor, source-program exporter, callback
admission or package migration. G2 (`87d586f`) added the executor, and the G3 checkpoint
below adds standalone authenticated source extraction. G4 adds schema27/parser7 package
export and explicit public admission. The legacy 1,222 Pure / 429 Unsupported dispositions
remain unchanged; complete supplied native originals remain **0/5**. The G1 tests are structural
and integration contracts, not original-source program parity or performance evidence.

Validation passes **51 scoped tests**: eight public program-schema tests, 11 independent
adversarial schema tests, nine compiler tests and 23 existing parser/factory contracts.
Strict all-target Clippy for data/engine and workspace formatting pass. All five portable
libraries compile for WASM, and native-only CLI normal dependencies contain no PoB/Lua.
The first broader data-suite compile caught a test fixture missing the newly required
append binding; that fixture was corrected and its 11 tests/lint passed again. The broader
full data regression subsequently completed with **183 passing tests across 26 targets**
(exit0, original session32274). Its original G1 binary/source identity is retained in the
ledger; this is not a G2 execution test. No fresh whole-build corpus run has occurred.

| Language stage | Current state | Completion gate |
| --- | --- | --- |
| G1: schema and compiler | Implemented; scoped tests and portability gates pass | Retain immutable owner binding, bounded validation and exact legacy package preservation |
| G2: execution | Native core, semantic/isolation tests and local validation pass; explicit primitive gaps retained | Retain raw-call contracts and close reached gaps before complete G3 source admission |
| G3: complete source lowering | Implemented; four complete original functions pass raw parity and seven live warm controls; 84 other emitted programs unproved | Preserve full inventory and raw/constructor proof |
| G4: package/parser integration | Published as `69db2cb`; local regression gates pass, hosted CI queued | Preserve four explicit permissions, source/public proof, package identities, corpus limitations and portability |
| G5: subsequent real dependencies | Future reached work | Add required effects/iteration/callable semantics without named callback handlers or weakened source proof |

**G2 implementation:** `CompiledParserPrograms::execute` uses a fresh bounded heap and
locals per invocation. Raw graphs preserve aliases/cycles, byte strings, IEEE values and
zero/Nil result cardinality. Helpers retain their own captures and share only invocation
handles. Numeric/iterator control state is independent of writable visible locals. Plans
and catalogs are immutable; input/definition writes remain explicit unsupported effects.

The intrinsic bridge supports original `createMod`, default/explicit-base10 `tonumber`, string replacement,
pattern iteration and dense append. Review repairs preserve method lookup before arguments
but noncallable-target errors after arguments; result packs are charged before allocation,
and byte comparisons consume shared scan work. Limits are cumulative logical allocation
units and work counters, not measured RSS. Failures publish no partial successful graph.

The paired oracle uses interpreted LuaJIT with JIT disabled. Authored IR covers source
loop/pack/alias/method/pattern behavior and original `createMod`; constant-template table
cases are explicitly lowered in test data. This is not automatic complete source lowering,
GemProperty/grantedExtraSkill parity, warmed-JIT evidence or throughput evidence. Runtime
modulo/power, raw legacy-factory calls and further dynamic effects remain visible gaps in
the [execution contract](typed-parser-programs.md#native-execution-boundary).

**G4 checkpoint after `a23510d`:** the normal parser constructor now consumes injected,
source/IR/definition-bound program admissions. Three public Special entries and their
captured helper use generic native instructions; 84 generated programs stay outside
public dispatch. The source/IR admission policy is separate from whole-function lowering.
Admissions record authored permission/evidence claims under the existing package trust
policy, not cryptographic proof of parity. Caller-authored programs can be deliberately
rebound as custom/unreviewed data; the CLI test proves that their changes affect output.

Failure-inclusive request accounting spans scans, native program calls, retry ordering and
bounded public graph conversion. Raw aliases/return packs stay lossless until the normal
wrapper applies first-two-result adjustment and public copy semantics; native parsing has
no result cache. Unsupported DTO shapes remain deferred, while cyclic public copies and
resource-limit failures retain ResourceError. Separate program/output/matching counters
persist across retries. Program source/resource errors keep their import provider
classification and source location; native-only consumers have no PoB/Lua host.

The authenticated schema27/parser7/program1 export retains all 27 other data sections and
all legacy parser definitions/dispositions. The complete generated catalog has 88 programs;
only four carry permissions. Two fresh public CLI exports from distinct working directories
are byte-identical to the installed package and have identical extraction evidence.
Previous schema26 bytes remain under `runs/typed-program-g4-baseline`.

| G4 evidence | Result / scope |
| --- | --- |
| Original public parser | Seven tests pass; 2,919 full output graphs (2,912 original-body pairs and seven explicitly authored-return wrapper controls), one paired source error, three DTO deferrals and one withheld-admission control. Includes four completed live warm controls; aborted trace attempts remain recorded. |
| Natural selection matrix | 2,879 of 2,883 candidates select the intended actual source callback. Three special names remain wholly unparsed and plain Sorcery Ward selects a different exact static entry; all four gaps are recorded, not counted as target parity. |
| Native runtime and integration | Full data suites pass 191 tests across 26 test-bearing targets (28 total, including zero-test library/example targets). 110 unique engine tests pass across runtime, source semantics and legacy contracts; 49 parser/extraction library tests and 50 existing source-regression tests pass. Full import/native suites pass 322 tests across 24 targets. Default/native-only item/data CLI suites each pass 23 tests; the source-inspection CLI target passes three. |
| Fresh supplied originals | Five imports/inspections/reference evaluations pass; all 110 frozen reference measurements match. Native evaluation still rejects three one-Skill and two one-SkillSet restrictions, so the corpus process retains exit1 and **0/5** complete native builds. |
| Controlled permission delta | On the same five inputs/package, disabling only admissions gives 598 reached parser calls and 27 parser stops. Enabled permissions reach 702 calls: 24 item records advance to assembly, one to rune reconstruction and one to a later parser stop. Final first stops are 109 assembly, four rune reconstruction, one affix-copy and two parser stops across 116 records. Native-only reviewed item output equals the default binary for the checked original. |
| Native parser benchmark | All 235,170 checked calls agree with serial output. Median checked calls/s: 2,407 at one worker, 4,544 at two, 9,079 at four and 33,333 at 16. This shared-machine parser workload includes output hashing, uses 702 reached calls/486 texts, and is not complete-build throughput. |
| Preparation/memory | Package read 6ms, load/validation 2.068s, immutable parser compilation 29.5ms. Windows working set after compile 221.9MiB, final 225.3MiB, process peak 665.9MiB during preparation. These are process measurements, not per-worker heap bounds. |
| Portability | Strict workspace/native-only Clippy, native-only dependency isolation and all five portable-library WASM checks pass. Formatting, whitespace checks and all 546 checked Markdown file links pass. |

Package SHA256: `875155eb794d8356a0c79cfd8b26cbe36f46f6dc2c01c408a2a0aa5eff7758d3`,
previous `7a509c7cffd6809154eb8e4ad8bfbedc8d3ae2707425e02b2f785264130ee3bf`.
The default release binary SHA is
`b9625cccce19f555bf9932d58cbe2cd0f799ac14fdc6b3e1a4a1e9ede1bb0fd6`;
173 release-source identities remain exact. The native-only control binary is separately
identified in `runs/typed-program-g4-native-controls/index.json`.

Evidence: `runs/typed-program-g4-validation.json`, `runs/typed-program-public-final.json`,
`runs/typed-program-shared-final.json`, `runs/typed-program-g4-data-validation.json`,
`runs/typed-program-g4-controlled-delta.json`, `runs/typed-program-g4-expectations.json`,
`runs/typed-program-g4-bootstrap-migration.json`, `runs/typed-program-g4-release-freeze.json`
and `runs/typed-program-g4-bench/validation.json`. Independent integration and CLI/source
reviews found no unresolved blocker. Failed initial fixture/lint/compilation attempts stay
in their original logs; exact-owner guards were retained and isolated legacy tests now
explicitly clear program permissions before changing their parser definitions.

G4 code is published on main as `69db2cb808804bfc1eebb3d5858bbf4e40bae7cc`.
Its exact-head [Windows/Linux CI run34555917045](https://github.com/Azaril/poe-optimizer/actions/runs/34555917045)
was queued at publication; this is not a hosted pass. Prior G3 run34552695729 was still
in progress with no observed failed job in `runs/typed-program-g4-prior-ci.json`.
The following update records publication only. The full native-parity goal remains active
and incomplete; this goal turn made progress by publishing G4.

**G3 checkpoint after `87d586f`:** the separate offline `parser_programs` API checks
pinned source bytes, reconstructs the entire original legacy catalog, and binds programs
only when it matches the caller's owner byte-for-byte, including signed zero. Primitive
and method-lookup identities are captured before construction and verified afterward.
The legacy extraction entry point and bundled schema26/parser6 package are unchanged.

Generic whole-function lowering inventories all 1,651 callbacks: **1,222 legacy Pure,
88 generated programs and 341 unsupported**. Only four generated functions have the
complete-original raw matrix below: GemProperty, grantedExtraSkill and both grant
forwarders (pinned IDs 286, 15, 738 and 739). IDs identify test evidence, never production
handlers. The other **84 programs are unproved**; no program gains public admission.
Unsupported syntax in any branch rejects the whole body, and missing captured helper
programs reject dependants. Lexical scopes, result expansion and permitted table writes
are generic; constructor collision forms and other language/effect gaps remain explicit.

The raw matrix passes **4,012 full-graph comparisons** (4,005 cold and seven warmed),
including all **961 gem lookup entries across all four targets**, alternate injected
lookups, alias/cycle/byte/nonfinite values, lazy paths and constructor shapes. Twelve paired
source failures retain the actual error class, innermost callback and location. Seven
warm controls require completed live original-body traces and reached constructor/callee
traces; they do not infer compilation from cache hits. Numeric dictionary keys outside the
validated catalog and opaque function observations have separate rejection/identity controls,
not native graph parity claims. Public Special packing, copying and cache behavior remain
an explicit G4 gate.

Independent review exposed a G2 assignment-order bug: `a, a = 1, 2` must leave `a` as 1.
The native executor now evaluates the full RHS pack and stores destinations right-to-left.
An independent interpreted-source regression reproduced the failure before the repair and
passes afterward, including repeated/missing/interleaved targets and an ordinary swap.
Multiple indexed source assignments remain unsupported; their source-only witness is not
counted as native parity.

Final local validation passes **135 tests across affected targets**: 61 engine library,
19 independent engine semantics, 45 PoB parser-related library and ten complete-source
matrix tests. The PoB set includes all nine lowerer, five primitive-authentication and four
public extraction API tests plus legacy parser/extraction regressions. Strict workspace
all-target Clippy, native-only CLI Clippy, five portable-library WASM checks, dependency
isolation, formatting and whitespace checks pass. Full workspace tests and fresh whole-build
corpus measurements were not rerun. No throughput claim is made; full native originals
remain **0/5**.

Evidence: `runs/typed-program-g3-validation.json`, `runs/typed-program-g3-engine-tests.log`,
`runs/typed-program-g3-pob-parser-tests.log`, `runs/typed-program-source-final2.log`,
`runs/typed-program-source-inventory-final2.json`, `runs/typed-program-source-final.json`,
`runs/typed-program-lowering-final.json`,
`runs/typed-program-auth-validation.json` and `runs/typed-program-source-data-review.md`.
The complete inventory retains every program, source dependency and unsupported reason.
Failed attempts remain recorded, including the invalid numeric-key fixture and the red/green
assignment regression. The first final source run predates root formatting; final2 binds
the final source set without overwriting earlier evidence.

Hosted G2 run34550304082 was still executing Test on Windows and Ubuntu at this checkpoint;
no failed step was observed. This is a nonterminal observation, not a CI pass. The current
G3 code has no hosted result yet.

**Resume point:** inspect G4 code run34555917045 and any observed failures, and implement
the accepted shared-model R1a foundation followed by R1b/R1c. The owner has answered the
previous B3 question in favor of structural correctness. Do not substitute another standalone named-helper campaign for real-build
integration. G5 language extensions should follow dependencies actually reached by the
paired Twister/Skeletal Sniper paths and the other supplied originals. Preserve the 84
unadmitted programs, current package/source evidence and caller inputs. J1 seed-data
investigation remains an optional opportunity; J2-J5 depend on the shared model and
complete relevant build evaluation. G4 grants no complete-build numerical admission.

G2 local validation passes **79 tests**: all 61 engine library tests (including 35 compiler/
heap/intrinsic/runtime contracts) and 18 independent source tests. The source matrix contains
58 complete raw graph pairs, ten source-error pairs and one explicit opaque-method gap;
all observations are interpreted. Four concurrent native threads repeatedly exercise two
catalogs with overlapping IDs and distinct captured values. Strict workspace all-target
Clippy, native-only CLI all-target Clippy, formatting, five portable-library WASM checks and
native dependency isolation pass. Full workspace tests and fresh real-build evaluations
were not rerun for this standalone executor checkpoint. No throughput claim is made.

Evidence for G2: `runs/typed-program-g2-validation.json`,
`runs/typed-program-g2-engine-tests.log`, `runs/typed-program-g2-clippy.log`,
`runs/typed-program-g2-native-clippy.log`, `runs/typed-program-g2-wasm.log`,
`runs/typed-program-semantics-final.json` and `runs/typed-program-executor-data-review.md`.
The original first test compilation failure (undeclared test-only Rayon import) was fixed
by using standard scoped threads; no runtime dependency was added. Independent review
findings and earlier fixture/lint attempts are retained in the ledgers.

Evidence: `runs/typed-program-data-validation.json`, `runs/typed-program-g1-validation.json`,
`runs/typed-program-engine-tests-final.log`, `runs/typed-program-legacy-parser-tests.log`,
`runs/typed-program-adversarial-final-tests.log`, `runs/typed-program-final-clippy.log` and
`runs/typed-program-portable-check.log`. Independent implementation-facing reads are retained
in `runs/typed-program-source-plan.md` and `runs/typed-program-compiler-oracle-review.md`.
Original source/caller bytes and historical package snapshots are preserved. G4
authenticated exports now establish package27/parser7; the G1 records remain historical.

The G1-G5 stages execute the owner's accepted direction and need no additional design
confirmation. Significant changes to that direction still require discussion. The separate
B3 model question is pending; these parser stages do not complete R1-R5 or authorize that
migration. Historical entries below retain their status at their original checkpoints.

## Fixed diagnostic expectation checkpoint

The new schema1 test manifest freezes the existing five original Pinnacle/level82 reports:
**93 finite, one positive infinity and 16 unavailable measurements**. Each case retains
original XML/report identities, backend/runtime evidence, the complete observed context,
selected actor/action records and independently authored selector strings. Raw unreserved
Life/Mana/Spirit observations remain outside the public metric set; Sniper Spirit=-67 is
preserved without inferring complete feasibility. No original or held-out build was
replaced, simplified or rerun to create this artifact.

The caller-controlled `scripts/check-build-expectations.py` reads an existing corpus index
and reports. It retains every required case and measurement, rejects missing/duplicate
rows and changed source/context/selections, and distinguishes unavailable/nonfinite values
from zero. Exact numbers are the default; an explicit diagnostic tolerance option uses
the manifest policy without relaxing existing native goldens. Results disclose observed
and reference backend/report identities. Different identity can match numeric observations,
which is not a source-authentication or complete-build parity claim.

Source-only resources are explicitly not compared by the public-metric checker. All
comparisons report `whole_build_parity=not_established` and native completion `not_assessed`.
The manifest therefore supplies a fixed starting reference contract, not a completed R3
gate. Generation checked all 110 rows against frozen raw values/availability and retained
34 source/calibration/caller hashes in `runs/breadth-expectations-generation.json`.
The generic checker and its negative tests are isolated from the production Rust API.

All **33 Python contract tests** pass: 32 synthetic cases and one tracked manifest/XML
integrity check. The saved reference reports match all **110/110 required measurements**
across 5/5 cases with exact numbers and report identities; the saved native reports remain
**0/5 matched, five blocked**. Neither comparison launched an evaluator. Independent
review found no remaining issue after typed schema/number and malformed-report fixes.
Both CI platforms now run this test suite. No Rust calculation, data package or original
fixture changes in this checkpoint, so prior numerical gates are not presented as rerun.

Validation/publication details are recorded in `runs/breadth-expectations-validation.json`
and `runs/breadth-expectations-publication.json`. Do not infer fresh native/reference
execution from an artifact comparison or promote its matched-case count to native breadth.

## Condition-trace evidence repair

Hosted String-checkpoint run34415712762 failed on Ubuntu because the warmed condition
control did not report a completed `ModStore.lua:281` trace. The call path still executed
Flag and returned the expected value. The test observer incorrectly accumulated historical
record events across aborted attempts and trace-number reuse, so its earlier successes
were insufficient evidence of which source functions compiled.

The repaired observer accepts only live completed traces, resets each recording attempt,
clears aborted/flush state and keeps observer readouts outside JIT compilation. Using
that observer reproduced the missing-wrapper observation on Windows. Original tagged
calls can reach LuaJIT's loop-unroll recording limit; an inner compiled consumer does not
prove its outer vararg wrapper compiled.

The control retains the original tagged GetCondition result and separately proves live
compiled GetCondition, plain Flag/FlagInternal and tagged EvalMod. It does not claim the
entire tagged chain compiles together. A new regression forces actual recording aborts,
proves reuse of an aborted trace ID without inherited GetCondition provenance, then
checks flush invalidation. Repeated readouts leave trace state unchanged.

All **14 scoped source tests** pass, including the existing numerical/source cases. The
two timing-sensitive controls also pass **eight fresh processes each**; strict target
Clippy, formatting and whitespace checks pass. Existing source, data and numerical
fixtures remain unchanged. Local evidence is in `runs/condition-warm-repair/`;
independent review and final publication bind the three changed test files. No successful
Linux rerun is claimed yet. The public failure annotation was sufficient to diagnose the
issue; the full job-log endpoint returned HTTP403 despite the authorized access attempt.

The same historical collector pattern was found and repaired in the separate mixed-store
harness; see its checkpoint below. Earlier query-pair numbers remain historical numerical
observations; their compiled-source interpretation is limited by the observer used then.

## Injected common item-assembly policy checkpoint

Main commit `d428915` integrates the isolated schema26 package, which adds
`ItemAssemblyData` and its immutable catalog. The new section is explicitly `PolicyOnly`: it supplies source-owned policy for a later
native item preparer, not an executable item or a complete build. Complete native
coverage of the five supplied originals remains 0/5. The shared real-build migration
and Twister/Sniper integration gates retain priority; do not start additional standalone
item families while that design direction is pending.

The catalog contains common row ordering/range rewriting, local and nil-config queries,
rune/grant rules, named compatibility, ordered attribute requirements and common slot
policy. Patterns, selectors, masks, names and numerical values are injected. It borrows
existing rune definitions, scalability, precision and keyword data rather than copying
those catalogs. Captured MatchAllMask and the live keyword table remain distinct. A
precision-only validator is shared with actor validation so caller-supplied precision
maps are bounded without imposing unrelated character quest assumptions.

Extraction authenticates 31 complete original method spans and their relevant live
bindings through the original item host. Provenance retains 113 source files and 114
module invocations, including the repeated module load. Whole-method authentication
is not complete branch lowering. Specialized weapon/armour/flask/charm/jewel slot
branches, mutable assembly state, provider production and actor calculations remain
unimplemented in this section. Neither constants nor source snapshots become computed
production input.

Independent review verifies that all 27 prior section values and digests are unchanged.
The package has 28 sections; modifier parser schema6 remains 1,222 Pure/429 Unsupported.
Package SHA256 is
`7a509c7cffd6809154eb8e4ad8bfbedc8d3ae2707425e02b2f785264130ee3bf`.
Two fresh public exports matched it exactly, with the existing preflight/preservation
and native-load controls passing. The updated CLI assertion retains the complete source
union, including the new section's provenance.

Validation passes: **244 scoped tests** cover 164 data (151 prior and 13 new), eight
source/host, 69 native and three public extraction tests. The merged tree also passes
**36 CLI item-inspection tests**, 18 with default features and 18 native-only, strict
workspace/all-target and native-only CLI lint, and formatting. Native-only dependency
isolation and five portable-library WASM compilation checks bind the unchanged production
sources after checkout newline normalization. These are scoped gates, not a fresh complete
workspace test run or browser execution.

The merged validation retains the exact identities of isolated source/export evidence;
it does not relabel an earlier binary as the merged binary. All 44 protected main git
blobs and both caller files are unchanged, and the original frozen worktree hashes match.
The initial cross-checkout raw-hash comparison exposed only CRLF/LF differences and is
retained with the corrected preservation audit. The PoB pin stays clean. No fresh
whole-build corpus execution or additional build admission is claimed.

Review and execution evidence: `runs/common-assembly-data-package-review.json`,
`runs/common-assembly-data-source-freeze.json`,
`runs/common-assembly-data-validation.json`,
`runs/common-assembly-data-portable-root.json` and `runs/common-assembly-public-export-validation.json`. Merged gates and source/protected-input checks are recorded in
`runs/common-assembly-integrated-validation.json`; final publication is recorded separately
in `runs/common-assembly-integrated-publication.json`.

## Mixed-store trace evidence repair

The old mixed-store observer accumulated aborted/reused trace records, like the condition
observer. Replacing only its collector reproduced missing required compiled functions on
Windows while the original numerical comparisons continued to pass. An attempted shared-VM
control then exposed inherited LuaJIT prototype blacklisting; both failed attempts remain
in the evidence record.

The repair uses six fresh, authenticated source VMs to prove live compilation of both
ModDB/ModList SumInternal and MoreInternal, GetCondition and EvalMod for explicit control
inputs. It keeps all **8,762 original paired observations**, including the 24 mixed-chain
pairs, unchanged. It does not claim that the complete mixed tagged chain compiles together.
The observer rejects aborted, reused and flushed provenance; readouts cannot create traces.

All **10 scoped tests**, strict target Clippy and **16 fresh-process control invocations**
pass. Independent review found no remaining issue. Evidence and exact three-file hashes
are in `runs/mixed-store-warm-repair/final.json` and
`runs/mixed-store-harness-independent-review.json`. Only test infrastructure changes;
source Lua, native adapters, game data and numerical fixtures remain unchanged. Hosted
Linux confirmation is still required; no local Linux pass is claimed.

## Concrete real-build API checkpoint

The [API proposal](real-build-api-proposal.md) specifies source occurrence, stable build
instance and private compiled-handle identities; independently selected saved views;
item-record versus slot-use identity; and partial native preparation diagnostics. It keeps
existing backend/evaluation traits and defers the candidate wire migration until the
ordered effect/provider search path needs it. The architecture question remains pending;
no production boundary or native admission changed at this checkpoint.

Independent review checked all five frozen XML/inspection/reference hashes and retained
11 focused API cases with 20 exact selected damage records in
`runs/real-build-r1-api-cases.json`. Sniper uses skill ID4/item ID2/tree position3/config ID1;
its two ring slots reference the same saved item. Twister's selected action index differs
from its group index. The cases also retain repeated grants, weapon context, unresolved
labels, unavailable damage and negative Spirit. They are reused observations, not new
source executions or numerical tests.

R1's gate now explicitly separates imported instances/selected views from the effective
actor/action graph. Unimplemented producers remain deferred; only implemented producers
can create effective nodes. R2/R3 must prove those producers and whole-build outputs.
This corrects an overbroad reading of the preceding R1 wording. The existing source
projections and identity catalog are inputs to migration, not work to repeat.

## Replacement-string and assembly-design checkpoint

`LuaPattern::gsub(subject, replacement, maximum, budget, limits)` returns exact byte
output and a substitution count. The optional maximum is already converted to i32;
Lua dynamic argument coercion and callable/table replacements remain separate operations.
A trailing percent emits NUL, percent plus a non-digit emits that byte, `%1` with no
captures means the whole match, and an unfinished capture errors only if requested.
The private matcher now exposes raw capture state; find/match still finalize all captures
at their original boundary. Pattern compilation remains immutable and source syntax
errors remain lazy. Output limits are checked and work charged before allocation/copy.

Validation includes 92 engine and 62 affected import tests, nine existing original
matcher/scan tests, five new original-C tests and 36 native/default item CLI tests:
**204 passing scoped tests** with no unresolved failure. The new matrix has 56,832
syntax/count, 27,648 all-byte, 22 explicit-quirk, 195 growth/capture and 16 warm-final
pairs. All eight warm cases show completed live host traces around real C calls.
Independent code review found no actionable issue. Two initial test-only issues (safe
Lua omitting debug, and a linted tuple type) are preserved with their passing repairs.
There is no reported native mismatch. This is scoped validation, not a full-workspace
or new corpus/full-build parity claim.

The new [assembly design](native-item-assembly.md) separates immutable injected policy,
engine mechanisms and import orchestration. It requires preparation-time aliases,
source-specific recursive copies, partial-error state and ordered dependency provenance.
Catalog extraction must preserve all 27 prior sections; metadata assembly must complete
every reached operation before changing admission. Active Bonded calculation, general
EvalMod, local-family formulas and the wider actor/action model remain separate work.

Historical checkpoints below retain the observations made at those times. Use the newest
resume point and evidence for current status.

The preceding Flag checkpoint is published as code
`e862651c0143ad1973b9cd50f1583d90059d47be` and docs
`c483db0d6547194019f7c683a143ebf1e447a502`. Its
[historical checkpoint](#closed-flag-factory-checkpoint) and
`runs/flag-factories-validation-final.json` retain the 421 scoped observations and exact
preservation evidence. Hosted run 34418412294 had passed formatting/lint and was testing
on both platforms at 2026-09-10 00:10:40 UTC, with no failed steps reported.

The preceding string checkpoint is published as code
`11e6f3906b30f5adba8e0847e6de6b84ea0f6894` and docs
`c79003a21082d2b0e86424755f518db2d2e9ed7d`.
Its [historical checkpoint](#bounded-string-factory-checkpoint),
`runs/string-factories-validation-final.json` and publication record retain all scoped
validation and preservation evidence. Hosted run 34415712762 had passed formatting/lint
and was testing on both platforms at 2026-09-09 23:12:45 UTC; no failed steps were reported.

The preceding ordinary factory checkpoint is published as code
`35960b6403c52be4a9ab2aed60c43fd3205ffe27` and docs
`114e86b3ad97112cca087e345c90e3049d92ab4e`. Its complete scoped validation and
preservation records remain in `runs/ordinary-factories-validation-final.json` and
`runs/ordinary-factories-publication.json`. See the
[historical checkpoint](#ordinary-factory-invocation-checkpoint).

The preceding pure Special checkpoint is published as
`27c11a2b253eacf2a21e9cd9c6905b07090c6013`, with resume update `5b56dd2`.
Its [code CI run](https://github.com/Azaril/poe-optimizer/actions/runs/34409563837) and
[docs CI run](https://github.com/Azaril/poe-optimizer/actions/runs/34409736342) had passed
formatting/lint on Windows and Ubuntu and were testing at 2026-09-09 22:04 UTC. No failed
steps were reported; hosted success is not yet claimed. The frozen callback worktree,
source/binary identities and original failed observations remain preserved in
`runs/callback-factories-publication.json` and its referenced validation records.

The complete native evaluator and broad joint optimizer remain unfinished. Native
whole-build admission is still limited to Spark/Mace; the five breadth builds still
reach three one-Skill and two one-SkillSet limits. The general **B3 actor/action/candidate
proposal remains unapproved**. Continue independent parser/item work without inferring
approval for that migration.

The preceding authored-affix checkpoint is published as `1d742b8d7c14ea2f9a76ce5bf70b4cc2640bfb00`.
Its frozen worktree, binaries and observations remain preserved under
`runs/affix-loading-worktree` and `runs/affix-loading-*`. Hosted status is independent:
[run 34398269518](https://github.com/Azaril/poe-optimizer/actions/runs/34398269518)
had passed formatting/lint and was still testing on both platforms at
2026-09-09 20:28:22 UTC. User authorization for CI log access remains in place.
Earlier full-log downloads returned HTTP 403; available public annotations supplied
actionable diagnostics. Preserve failed historical runs rather than relabeling them
based on a later local repair.

Earlier checkpoint entries below retain their original validation observations. Use this
resume point and the newest evidence records for current status.

The preceding checkpoint added native defence headers/base buffs and reference item-database
readiness. Native loading code is `8781c7c959ac548ea74d364f0207cb0e614633dc`;
the reference repair is `45ed37149b48a634613bfaf36b6ac1e656040a84`, integrated as `d213a1fc`.
See [readiness](#reference-item-database-readiness-checkpoint),
[base buffs](#base-flaskcharm-buffs-checkpoint) and
[defence headers](#defence-header-data-and-state-checkpoint).

Previous main checkpoint: **native structural modifier parsing**, code `415ab742`,
test-fixture repair `765ae933`, and resume update
`15525b1ec25e4877024e20a06efecb812c03dc9d` on main. Local reconciled validation is complete:
**1,066 passed, zero unresolved failures across all 140 current targets**, plus **115
native-only CLI tests**. Nine ignored direct-entry helper tests are exercised by passing
parent subprocess tests. All 360 final input fingerprints match. Original failures and
current-code replacements remain in `runs/parser-workspace-validation-final.json`.

Exact repair-publication [Windows/Linux CI run 34381171078](https://github.com/Azaril/poe-optimizer/actions/runs/34381171078)
passed the complete workflow on Windows and Linux (confirmed 2026-09-09). Preserve
the preceding pre-repair failed run 34379251966 separately. Full native build support remains limited to the admitted
Spark/Mace pipelines. See the [parser checkpoint](#general-modifier-parser-active-checkpoint).

The preceding **general item formatting** checkpoint is published as
`48983fa7f57ec66d26ab9e6525a01fd3b97e55f5`.
[Exact-code CI run 34327347326](https://github.com/Azaril/poe-optimizer/actions/runs/34327347326)
passed on Windows and Linux (confirmed 2026-09-09).

The preceding item definitions/loading implementation is published as
`43251c748ce735783bb0df6357f54f16d4134e35`. Its
[CI run 34320179629](https://github.com/Azaril/poe-optimizer/actions/runs/34320179629)
failed on a new Rust/Clippy lint before tests. Published repair
`1f1ad1f520db85045888f479edb6263481df5c95` pins Rust 1.98.1 and resolves that lint.
All **941 local workspace tests pass** (nine intentional helper tests ignored by direct
execution). [Repair CI run 34321961512](https://github.com/Azaril/poe-optimizer/actions/runs/34321961512)
passed the complete workflow on Windows and Linux (confirmed 2026-09-09).

The preceding mixed ModList/ModDB query checkpoint is published as
`5f8b70e220e2bd19ed79d0e3b6979a2347970ab2`, with resume update `316ad8c`.
[Exact-code CI run 34274132680](https://github.com/Azaril/poe-optimizer/actions/runs/34274132680)
passed on Windows and Linux. Native numeric and condition programs preserve every store
kind in a parent chain and enforce matching query contexts; shared SUM retains child-first
error traversal, grouped arithmetic and bounded allocation-free scratch.

The preceding condition checkpoint is published as
`d09252bc22a2f0a8788274e52b3275ae1885f5d6`, with resume update `ba8343c`.
[Exact-code CI run 34271992799](https://github.com/Azaril/poe-optimizer/actions/runs/34271992799)
passed on Windows and Linux. The broader five-build corpus still needs
general actor/action, item and calculation pipelines; the shared query implementation is
one component of that work.

The preceding item-source checkpoint is published as
`5b2ac700c01412b1778194b4b94ed291a6282c36` with resume update `ec17180`.
The subsequent diagnostic run exposed the extraction test's explicit 30-second deadline
on both platforms (Linux 30.02 seconds, Windows 30.04 seconds). Published fix `c7fb7fe`
increases only that functional reproducibility test's budget to 120 seconds; its three
cases pass locally, including two exact 23-section extractions and native reload.
Production timeout defaults and timeout/kill tests are unchanged.
[Fix CI run 34269405262](https://github.com/Azaril/poe-optimizer/actions/runs/34269405262)
passed on Windows and Linux. Local diagnosis is in
`runs/ci-extraction-functional-fix-evidence.json`. Earlier checkpoint entries below retain
the information available when they were published; the log request is no longer needed.

The preceding **skill source projection and injected identity catalogs** checkpoint is
published as `19666a0ad754ac5514553a71138a568729022d2e` on main, with resume update `7318300`.
[Exact-code CI run 34257699126](https://github.com/Azaril/poe-optimizer/actions/runs/34257699126)
completed with Linux failure and Windows cancellation. Its actual failing assertion is not
available through the current public API/browser access; additional local checks pass.
Its local validation remains recorded below. The source/catalog CLI preserves all saved occurrences
and uses selected data for exact, fallback, ambiguous or unresolved identity evidence.
No additional build is claimed as native-supported. See the
[skill source contract](skill-source-and-identities.md).

The preceding checkpoint, **source build containers and shared MAIN admission**, implemented;
locally validated and published as `b109b941dd7d6f7f928b69e27d7e651ffd507415` on main.
[Windows/Linux CI run 34253146642](https://github.com/Azaril/poe-optimizer/actions/runs/34253146642)
passed on Windows and Linux. Arbitrary caller inputs can be inspected
without a data package or PoB runtime. Proven-inert Import/Party/Calcs/TreeView shapes now
pass a shared native gate while unknown/effectful content remains rejected. The five
original builds advance to the existing one-Skill/one-SkillSet limits; this is structural
progress, not broad numerical parity. See the checkpoint table below. Full native
replacement remains unfinished.

The preceding checkpoint, **injected configuration definitions**, implemented and locally validated. The portable schema-12 catalog, complete authenticated extractor,
original-source default-state oracles and caller-driven CLI lookup are implemented. All five
original builds retain exact source values; definition recognition does not grant mechanic
support. Numeric section content and the pinned tree remain unchanged. Code is published as
`87bf0056209653d9b46d922477e9cfe920aeb6b4` on `origin/main`. Exact-code
[Windows/Linux CI run 34249708752](https://github.com/Azaril/poe-optimizer/actions/runs/34249708752)
passed on Windows and Linux. Full native replacement remains unfinished.

The preceding checkpoint, **breadth configuration source projection**, is implemented and
locally validated. The shared reader, original-PoB parser comparisons, injected-definition inventory
and caller-driven inspection are complete. Native/search scalar readers share exact source
semantics; all five corpus projections and fresh PoB runs succeed. Native still rejects the
broader build layout. Preserving unknown configuration does not admit its game effects;
broad skill/actor/build support remains pending. Code is published as
`0da5c5c288442f1ac11cc5ffea0c948813dc6d9e` on `origin/main`. Exact-code
[Windows/Linux CI run 34243706906](https://github.com/Azaril/poe-optimizer/actions/runs/34243706906)
passed on Windows and Linux.

The preceding **shared ActionSpeed and ordinary direct-action timing** checkpoint is implemented.
Data/extraction, shared Rust calculations, import/native adapters, CLI integration and
complete-build parity, complete target validation and isolated release measurements pass.
Published code: `caa56f4374da864acfb99cc19060ed356bd35105`. Its Windows/Linux CI run
`34238329480` passed on Windows and Linux; broad native build support remains unfinished.

The completed **Body Armour and shared movement** checkpoint is published as
`56dcbc500ebdf3f2011663462c4c0a0a794aaabe`. Its local validation and release measurements
pass; exact-code Windows/Linux CI run `34228787060` has passed on both platforms.
Next after action timing: **breadth of validation and data-driven build admission**, before
another individual-skill port. The user's five new line-delimited imports are the initial
corpus, not a replacement for the original independent reference fixtures.

The preceding **local armour equipment and rating objectives** checkpoint is published as
`e42760d233e37d75fcc04b07e6a30634fb7fdae9`. Its exact-code CI run `34221294167` passed
on Linux and failed a Windows test because an LF-only fixture edit silently missed CRLF
input. That failure was reproduced locally and repaired in the current checkpoint; see below.

The preceding **shared receiving defences and resistances** checkpoint is implemented,
locally validated and published as `f1d8a1c404a8c3dc77b11d7ff4410a241b0d902b`.
Exact-code Windows/Linux CI run `34214745950` has completed successfully on both platforms.
Schema 8 moved defensive passive contributions into ordered actor records; configuration,
weapon, amulet and passive sources use the same receiving stage.
The preceding **normalized passive and equipment source assembly** is implemented,
locally validated and published as `a2586a89a8160ae0b31d42d23ed921cb3b5823b2`.
Exact-code Windows/Linux CI run `34208805533` has completed successfully on both platforms.
The preceding **shared actor attributes and maximum resources** checkpoint is implemented,
locally validated and published as `4df9a4fcefe271011cd612fc1f8c7bc43d863c66`.
Exact-code Windows/Linux CI run `34199225766` passes on both platforms. Schema-6 injected actor data, shared Rust
preparation, source configuration admission and CLI problem 6/report 7 are complete for the
bounded scope below. The preceding local-weapon code `da168e94410730b8ee97764d54feb126484d12d3`
passes hosted Windows and Linux CI run `34193804854`.
The full implementation-plan/native-parity goal remains active.

Native support remains limited to the documented Spark/Mace pipelines. The supplied
minion build, general equipment/skill/modifier pipelines and full PoB parity remain
unfinished. Source revision, tree/full source snapshot and six independent goldens stay fixed.
The user intentionally replaced `example.import.txt` with five imports on 2026-09-08;
preserve these bytes and retain the original decoded build/reference fixtures. Actor/item grammar and effect values are injected data;
supplied synthetic rare rolls do not certify affix-tier or acquisition legality.

This is the living record of delivery order, implemented behavior, validation, unresolved
work, and the next session's starting point. [Design](design.md) defines the intended system
and acceptance criteria; [execution and interfaces](execution-and-interfaces.md) and the
[calculation boundary decision](calculation-boundary.md) define
runtime and application contracts. The [PoB investigation](pob-integration.md) and
[prior-art review](prior-art-and-product-review.md) retain dated evidence and rationale.
Update this document at progress checkpoints, rather than adding implementation status to
the design documents.

## Resume here

1. Read the breadth dashboard and [real-build rollout](real-build-rollout.md). The shared
   model direction is accepted. R1a source ownership, R1b selected views and the
   [R1c native entry point](native-preparation.md) and [R2a authored skill loading](authored-skill-preparation.md)
   are implemented; general effective producers and all five complete native evaluations remain unfinished.
2. Check the working tree and exact-head CI. Preserve frozen worktrees and terminal
   validation records. Fix actual hosted failures without weakening source evidence;
   local passes do not replace pending Windows/Linux results.
3. Preserve the PoB pin, original caller imports, all saved sets, numerical goldens and
   frozen evidence. Keep native calculation independent of Lua and subprocesses. Host
   lineage is assigned at import; portable callers use explicit lineage entry points.
4. Advance R2 preparation on the original Twister and Skeletal Sniper views together.
   R2a authored loading and per-build definition overlays are implemented. Next map the original
   root lifecycle and effective configuration producer, then connect reached item/passive,
   actor/action and provider prerequisites in source order. Preserve configured callback
   semantics through the accepted typed-program seam. Reuse shared catalogs and kernels, and compare
   executable stages with complete original-source methods. Do not add another closed profile.
5. Preserve and extend the fixed diagnostic expectation manifest through explicit revisions.
   Its saved-artifact comparisons do not rerun backends or establish native coverage. Add
   explicit mapping cases and full required resource semantics before claiming R3/R5.
6. Update this dashboard and exact per-build blockers at each integration checkpoint.
   Verify changed interactions and exported source before expanding search or making
   performance claims. UI autocomplete will use loaded definitions and a derived index;
   DuckDB is deferred. Historical checkpoints below are evidence, not a request to repeat
   completed suites or postpone real-build integration.


## R2a authored skill preparation checkpoint

Implemented a shared, owned native stage for original authored Load/LoadSkill,
ProcessSocketGroup/FindSkillGem and reached level/requirement helpers. Saved alternatives,
duplicate set effects, table/string key distinction, source text-array behavior, partial
failure prefixes, explicit overrides and unresolved/ambiguous identities remain observable.
Completing this stage does not activate item/tree grants or construct effective actors/actions.
Triggered cost replacement is representable through alias-preserving per-build overlays;
its private unit test is not evidence that authored XML executes a triggered provider.

The injected schema 28 package is 26,285,617 bytes, SHA-256
`35f5577fb9377ea293729cf81f766b9ad09addd1945d823b72a55d5dab2c3231`.
The new section digest is
`a39dcaec184eda8c19d0ee5ccae3a7f5731e4a9bb312e0128a6298aff09a9357`.
All prior sections are exact; original schema 27 is backed up under
`runs/r2-skill-preparation-schema27-backup`. Original imports, golden fixtures and PoB pin
`3887ae68a6a6b8bb7b41d1b61998f1aa184201e4` remain unchanged.

Source traversal order differs across independent Lua constructions. The runtime artifact
therefore stores canonical semantic rows and explicit ambiguous candidates. Extraction
sidecar schema 2 retains actual complete traversal and winner observations, validates their
permutations and replay, and keeps all other evidence stable. Unknown metatable behavior
rejects extraction instead of silently replacing source lookup semantics with raw fields.

Candidate review reproduced a real discrepancy: a support introduced after baseline
preparation could calculate in the fast path even when fresh document preparation rejected
its injected hidden/level-clamped definition. The shared cold admission helper now consumes
support-only variations produced by the existing import materializers. It caches per-support
or per-loadout success/errors, preserving unrelated candidates. Temporary XML/stage ownership
is discarded before repeated calculation; neither a Cartesian document cache nor a second
loader algorithm was introduced. New tests compare both candidate APIs with fresh documents
and confirm unaffected candidates still evaluate after a rejected support.

Evidence: `runs/r2a-native-regression.log`, `runs/r2a-import-regression.log`,
`runs/r2a-cli-default.log`, `runs/r2a-cli-native.log`,
`runs/r2-skill-preparation-tests/validation.json`,
`runs/r2-authored-skills-validation.json`, `runs/r2-skill-preparation-validation.json`.
Final integration evidence is recorded in `runs/r2a-integration-validation.json`;
`runs/r2a-publication.json` records the published head and its hosted CI observation.
The optional broad data suite was deliberately interrupted after completed unchanged-mechanic
targets; it is not claimed as a full passing run. Focused loader/catalog regressions are the
required data checks for this checkpoint.

**Next resume:** finish/check exact-head CI, then R2b effective configuration and root load
ordering. Compare complete original producers on Twister and Skeletal Sniper together and
preserve the other three original paths. Continue through item registration, passive/grant
providers and actor/action construction before R3 full outputs. Keep the 22 × 5 diagnostic
expectations, explicit mapping variants and source-selected unavailable measurements intact.
No complete real-build numerical evaluation or whole-build parity has been added by R2a.

## Toolchain lint repair - local validation complete

The preceding goal turn was progress: item catalog/loading code `43251c7` and resume update
`e224002` were published. This turn started from a clean worktree; 26 protected files were
captured in `runs/item-formatting-baseline.json`, with the schema-14 package copied to
`runs/item-formatting-original-game-data.json`. The whole native parity goal remains active.

Exact-code run 34320179629 failed at Lint on both operating systems; later tests did not
run. Public annotations expose only exit codes, the public log download returns HTTP 403,
and the authenticated browser helper is unavailable in this runtime. User authorization
for log access remains in place. Local `rustup check` identified stable 1.98.1 versus the
previously installed 1.93.0. Running the exact full-workspace lint command with 1.98.1
reproduced `clippy::collapsible_match` at the new item's FindImplicit transition.

Repair code is published as `1f1ad1f520db85045888f479edb6263481df5c95` on main.
[Exact repair CI run 34321961512](https://github.com/Azaril/poe-optimizer/actions/runs/34321961512)
passed the complete Windows and Linux workflow (confirmed 2026-09-09 in
`runs/parser-prior-ci-34321961512.json`). The earlier observation is retained in
`runs/ci-198-publication.json`.
Local full-workspace session **51809 completed with exit 0**: 941 tests passed across 124
target results, with nine intentional helper tests ignored by direct execution. Do not
restart it. `runs/ci-198-workspace-tests-final.json` records the exact commit, command and
counts; `runs/item-formatting-pinned-workspace-tests.log` retains the full results. The
known lint failure is resolved locally and on both hosted lint stages; hosted test results
remain pending.

The transition now uses an equivalent guarded match arm. The project toolchain is pinned
to 1.98.1 so local and CI formatting/lints are reproducible; crate MSRV declarations remain
unchanged. Formatting and strict full-workspace lint pass on the pinned toolchain. Full
workspace tests pass in `target/ci-stable-198` (completed root session 51809). Pinned native-only CLI strict lint and all five WASM libraries pass with source hashes
unchanged (`runs/ci-198-portable-summary.json`). The existing bounded failure-annotation
helper now also wraps both CI
lint steps, with a generic command-failure title, so future lint diagnostics are available
through public check annotations.

Evidence: `runs/item-formatting-prior-ci.json`, job-specific annotation JSON files,
`runs/item-formatting-toolchain-install.log`, `runs/item-formatting-stable-clippy-before.log`,
`runs/item-formatting-stable-clippy-after.log`, `runs/item-formatting-pinned-workspace-tests.log`.
Rust 1.98 additionally surfaces a pre-existing Windows mixed-CRT linker warning: LuaJIT
archives contain LIBCMT directives while the UTF-8 shim and Rust use the dynamic CRT.
Earlier artifacts have the same split. No runtime failure was demonstrated; native-only
deployment excludes both archives. A follow-up should make the vendored LuaJIT build honor
the target CRT while preserving static Lua linkage. Do not suppress the warning or switch
only the shim's CRT (`runs/ci-198-windows-crt-artifacts.json`). Failure-annotation smoke
checks preserve exit 17, percent escaping and a successful command without false errors.
All 26 protected files remain exact (`runs/ci-198-preservation.json`).

Formatter development is isolated in the detached worktree
`runs/item-formatting-worktree` at repair commit `1f1ad1f`; the main checkout stays frozen
through the completed local regression session 51809. The worktree has a separate
local shared-object PoB clone at the same pinned revision, not a junction to main. Keep the validated source/data evidence intact when integrating this checkpoint; track
hosted test status separately from the completed local and lint repair gates. Keep
the existing restricted item_formatting section unchanged and inject a separate complete
scalability catalog. Preserve ordered raw labels and their partial assignment semantics;
unknown labels are original no-ops. Reuse the existing complete actor high-precision table
rather than copying it. The implementation-ready source audit is retained in
`runs/item-formatting-data-audit.json`: 15,090 keys, 3,321 empty entries and 12,040 ordered
capture records; all 33 dispatcher assignments and 14 ignored labels are explicit.
Its proposed catalog adds 77,073 JSON values and approximately 2.4–2.7 MB, within current
package limits. Native runtime proofs are still required. No actor/action/candidate
migration is included in this work.

## General modifier parser active checkpoint

Started after `48983fa` on 2026-09-09. The implementation and source-validation work is
progress toward reusable parsing for items, passives and skill effects. It does not yet
provide complete native builds beyond the existing Spark/Mace pipelines. The B3 general
actor/action/candidate model proposal remains unapproved; no migration is included here.
A concrete decision request was sent to the user on 2026-09-09; keep dependent migration
work pending until a reply arrives. Parser/item implementation can continue independently.
The [parser contract](modifier-parser.md) describes the boundaries and end state.

Work is on main in `C:/code/poe-optimizer`. The detached `runs/item-formatting-worktree`
remains frozen at `48983fa`, retaining the preceding validated binaries. The protected
baseline and original schema-15 package are in `runs/parser-baseline.json` and
`runs/parser-original-game-data.json`. Caller input, source/tree manifest, Cargo files and
independent goldens remain unchanged. All 25 preceding package section values/digests are
exact; only the intended package migration changes its protected baseline bytes.

### Implemented behavior

- Schema 16 (`poe2-native-profiles-v16`) adds the complete `modifier_parser` catalog:
  28 dictionaries / 10,027 rows, 19,634 table objects, 1,645 Lua closures, six named C
  primitives, ordered upvalues, 4,390 lexical declarations and all final winners.
  Complete generator dependencies retain all raw gem base-name assignment candidates,
  including Lightning Bolt (2), Mace Strike (3) and Spear Stab (2). No build/profile
  whitelist or arbitrary winner resolves these ambiguities.
- Portable package limits are 32 MiB and two million JSON values; extraction allows a
  33 MiB package/evidence envelope. The 26-section package contains 21,598,614 bytes,
  SHA-256 `27a1ca4d66cfb02333ad8b5145afeb07fc999c5f33791a6d530fed9a8f3a32da`.
  Fresh F/G extractions reproduce both package and evidence bytes, using 122 authenticated
  sources. Evidence SHA-256 is `ada9428a29b233cfbaa99f875dd2c092743001746c5385cf7664513ed35147c5`.
  C primitive identity does not serialize internal C closures or grant callback execution.
- The pure byte matcher implements original LuaJIT pattern/capture behavior, including
  lazy source errors, NUL/plain distinctions and source capture/depth limits. Matching
  work, compiled memory and output expansion have separate bounds. Portable `tonumber`
  and signed-low-word `OR64` preserve independently observed source behavior.
- Compiled scan tables retain source selection priority. Exact ties require equal
  captures and bounded exact-bit payload equivalence, including shared DAGs, NaN and
  signed zero. False winning payloads and later pattern errors retain source order.
- `CompiledModifierParser` implements ordinary forms, static rules, sparse contribution
  merging, tags, positional wrappers and the public copy/retry behavior. All raw rows
  precede wrapping. Compiled rules are immutable and shareable between workers; execution
  has no Lua runtime, process, I/O, hidden fallback or shared mutable global cache.
- `NativeModifierParserProvider` connects formatting and parsing to native item loading.
  Nested dense numeric-only tables match the existing metadata array representation;
  sparse/mixed/empty tables stay tables. Wrapper output expansion is charged before cloning,
  including player-tag lists, standalone tags, aura transfers and enemy copies. Callback
  payloads, non-finite values and non-UTF-8
  output stop explicitly at the current metadata seam. The existing numerical profile
  grammars remain unchanged. CLI inspection uses caller input and injected catalogs.

### Validation and publication

| Gate | Current result |
| --- | --- |
| Independent original parser/pattern/number tests | 23 tests pass; 8,789 structural comparisons, zero mismatches; 286 explicit deferrals |
| Numeric primitives | 64,283 cold `tonumber`; 6,296 cold and 18,888 warmed OR64 comparisons |
| Pattern/scan behavior | 100,920 adversarial finds; 11,264 byte classes; 54,064 dictionary finds; 7,200 capture checks; 512 scans |
| Corpus parser coverage | 116 items / 727 source parser requests / 506 unique texts: 437 paired, 69 deferred |
| Static callback descriptors | All 33 observed callback-bearing outputs compared; no observer skips; execution remains pending |
| Original parser session semantics | Three independent tests pass: shared composite-name mutation, injected-error persistence without cache insertion, and returned jewel callback reachability/identity |
| Native contracts | 17 primitive, six scan, nine parser contracts and six wrapper resource regressions pass; strict Clippy passes |
| Data model/extraction | 105 tests across 16 targets pass after updating one stale schema assertion; source reproduction and strict Clippy pass |
| Import/provider integration | Five import contracts, nine formatting contracts and seven independent provider tests pass. 1,597 broad provider outputs match with one explicit DOUBLED deferral; 24 combined LF/CRLF cases and all 116 corpus trace prefixes pass, including 28 genuine final preassembly states. |
| Native-only inspection/config CLI | 24 tests and strict Clippy pass; stale parser-stop assertion updated to assembly, preserving authored text |
| Portable/dependency gates | All five WASM libraries compile after the final wrapper/header fixes; native-only normal dependencies exclude PoB/Lua. Final strict workspace/native-only Clippy and formatting pass. |
| Fresh caller corpus | Five imports/inspections/reference runs succeed; all 110 reference measurements and native rejection reasons unchanged |
| Corpus tooling | 29 tests pass; bounded data copy now accepts the schema-16 package size |
| Native-only complete CLI regression | Session 34129 completed with exit 0: all 115 tests across 45 target results pass, none failed or ignored. Final binary SHA-256 `cdf38223e457a83167a448517d4781c8142416f18c23b2511b6dbf7d62833665`. |
| Full workspace/current import regression | Session 37729 is terminal with its three original failures retained. Final identity-aware reconciliation covers all 140 targets: 1,066 pass, zero unresolved failures, nine explained helper entry points. All 223 current-code import tests are included without duplicate counts. All 360 final fingerprints match. See runs/parser-workspace-validation-final.json. |
| Publication | Code `415ab742`, fixture repair `765ae933` and resume update `15525b1e` are published on main. Exact repair CI 34381171078 is running after passing formatting/lint on both platforms; original run 34379251966 is tracked separately. |

Final item diagnostics preserve all 116 inventory items, 486 range instructions, 15 saved
skill/equipment sets, 200 groups, 541 gem occurrences, 16 passive specs and 21 jewel
assignments. All items remain pending: 31 assembly, 31 base compatibility, 18 parser,
15 base buffs, ten crafted affixes, seven rune reconstruction and four unique database.
The previous checkpoint had 51 parser and four assembly stops.

An independent preassembly comparison exposed defence-header precedence: original
`Item.lua:835–854` consumes Armour/Evasion/Energy Shield/Ward headers before the later
`hidden_specs` branch. The loader cannot yet retain its full `armourData`/base-rebinding
operation, so it now stops explicitly at `BaseCompatibility`, before falsely marking
hidden specs. Six header spellings have independent source witnesses and a focused import
regression. The first corpus run (`runs/parser-corpus-final`) predates this fix; the final
reviewed run (`runs/parser-corpus-reviewed`) uses the frozen corrected binaries and reports
the counts above. Its 110 reference measurements and all represented build/context/coverage/
warnings remain bitwise unchanged. All five native builds still reject at the existing
one-Skill/one-SkillSet boundaries. Runner exit 1 represents these expected native rejections;
source inspections and PoB runs succeeded.

Evidence: `runs/parser-data-final.json`, `runs/parser-data-tests-final.json`,
`runs/parser-data-protected-files.json`, `runs/parser-oracle-final-summary.json`,
`runs/parser-number-native-checks.json`, `runs/parser-ordinary-contracts-final.json`,
`runs/parser-cli-validation.json`, `runs/parser-native-cli-test-ledger.json`,
`runs/parser-wrapper-review-fix.json`,
`runs/parser-import-final.log`, `runs/parser-provider-public-final.log`,
`runs/parser-session-audit-final.log`, `runs/parser-import-all-final.log`,
`runs/parser-extraction-cli-repaired.log`,
`runs/parser-corpus-reviewed-validation.json`, `runs/parser-final-preservation.json` and
`runs/parser-corpus-reviewed/index.json`. These ignored local ledgers supplement reproducible
committed tests; they are not independent game truth.

### Resume point

Independent provider/source integration, native-only validation and full current-target
reconciliation pass. Inspect the exact repair-publication CI run 34381171078 and preserve
any failure evidence. Do not present pending hosted gates as passed.
Keep successful terminal results; do not restart old or still-running sessions.

Next, implement selected callback operations and explicit source parser session state.
`DOUBLED` mutates shared composite-name data: cached earlier text retains its old result,
while fresh text sees the mutation. An explicit bounded session overlay/cache must preserve
that behavior without mutating the injected catalog or sharing state between workers.
Callback spans/upvalue graphs are definitions, not native implementations. Keep native
pending, source errors, resources and ambiguity separate; never select a lower-priority
rule merely because the winner is unsupported. Original-source state/copy/alias tests must
remain independent of native code and extraction.

The next defence-header slice is scoped by `runs/parser-defence-header-next-audit.md`.
Inject the original header-to-armour-key mapping and reuse `base_aliases.armour_header_rewrites`;
add optional nested armour data with the original reparse lifetime. Rebinding precedes
numeric parsing and changes only base name/reference. All three authored Two-Toned base
references are absent at the pin: preserve the resulting nil base instead of rejecting
those definition references or keeping the prior base. Nil numeric assignment removes a
key while retaining its table. Preserve the later explicit/implicit-line path after a
known header; do not skip it. Independently test ordering, injected replacements and
reparse behavior before removing the current pending dependency.

Then complete required item dependencies (unique/rune/affix/base-buff and assembly
operations), effective skill/grant/action resolution and numerical producers.
Numerical support, item assembly and complete-build admission remain separate gates. The
full native-parity and broad joint-search goal remains active; this checkpoint completes
neither the B2 mechanism matrix nor the proposed B3 architecture migration.

### Formatting extraction fixture memory repair

The full workspace found an mlua `MemoryError` before the formatting extraction assertions:
the legacy fixture copied the entire schema-16 package into a bounded Lua state. Original
`game_data_extract.lua:605–606` consumes only `records.actor.modifier_rules` and
`records.item_modifier_rules`. Commit `765ae933` supplies exactly those collections. All six
malformed-shape mutations and three format-result assertions remain unchanged; the production
Lua cap stays 128 MiB. The focused test, strict PoB all-target Clippy and workspace formatting
pass. Evidence is `runs/parser-formatting-extractor-repaired.log`,
`runs/parser-pob-clippy-after-fixture-repair.log` and
`runs/parser-format-after-fixture-repair.log`.

Both isolated worktrees carry the same repair as `e602ba3`, without changing their native
loading code or package bytes. The extractor fingerprint includes this Rust file's text,
so future extraction evidence has a new extractor identity even though the generated game
data and production extraction behavior are unchanged. Preserve earlier evidence under its
original identity; do not relabel old binaries or rerun unrelated numerical suites.

## Defence-header data and state checkpoint

Implemented and locally validated in `C:/code/poe-optimizer/runs/defence-header-worktree`,
branch `codex/defence-headers`. Code commit `2d13d4f` preserves all implementation edits;
merge `ecb9230` also retains the published parser resume update. The separate PoB clone
remains clean at the same authenticated pin. Main production code stays frozen while its
parser regression finishes. No actor/action/candidate migration or additional native
numerical build admission is included; the B3 decision request remains pending.

Schema 17 (`poe2-native-profiles-v17`) advances item-loading data to schema 2. Injected
`ItemLoadingPolicy.defence_header_keys` supplies header/key identities, and borrowed
catalog accessors reuse `base_aliases.armour_header_rewrites`. Fresh independent extractions
reproduce the package and evidence bytes exactly; all 25 other section values/digests and
all earlier item-loading fields are unchanged. The package contains 21,598,920 bytes,
SHA-256 `220ea3decf5ee887c2b2d7890be253837f21e6c6bf45424e07a85a8573000acf`.

`ItemState.armour_data` distinguishes absent and empty numeric tables, retains unrelated
keys and survives reparses. Nil numeric assignment removes its key after creating the table.
Base rewrites run before number conversion and update only the name/reference, including
retained names whose current reference is absent. Missing targets stay absent. A known
header still follows later explicit/implicit parsing when the source requires it.
`AssemblyOutcome.armour_data` carries bounded, validated `Preserve`, `Clear` or `Replace`
updates. These copied display values remain loading evidence, not calculated item ratings.
The worktree extends [the item-loading contract](item-source-and-loading.md) with these
state semantics; the main contract receives that section at integration.

| Gate | Result |
| --- | --- |
| Import contracts and provider regressions | 39 pass: 25 loading, nine formatting, five parser-provider tests; strict Clippy passes |
| Data and extraction | 17 pass: 13 item-loading and four original-source extractor tests; independent package/evidence reproductions match; strict Clippy passes |
| Independent original item tests | 24 pass: five new defence-header, 12 existing loading and seven parser-provider tests; strict Clippy passes |
| Source matrices | 64 numeric/alias, 18 explicit/implicit continuation, nine assembly lifecycle and three stale-base reparse cases; authoritative missing references and separately injected successful rebinds |
| Corpus original-source loader comparisons | All 116 executed trace prefixes match; 155 parser and 153 formatting prefixes; 29 complete preassembly states |
| CLI | 16 default-feature tests pass, covering extraction/reproduction, inspection and native-data exports. New injected-header CLI test also passes without PoB; strict affected-target Clippy passes in both configurations |
| Portable and static gates | Five WASM libraries compile, native-only runtime excludes PoB/Lua, final workspace formatting passes; independent lifecycle/resource review has no actionable findings |
| Fresh five-build regression | All imports, inspections and PoB runs succeed; all 110 measurements and represented contexts, coverage, warnings and non-timing raw diagnostics remain bit-exact; native rejects the same three one-Skill and two one-SkillSet cases |
| Documentation | 46 documents / 490 local links checked with no broken targets or anchors before this checkpoint update; recheck after final integration |
| Publication | Defence-header code remains local. Complete preceding parser reconciliation and inspect its exact CI before integration/push |

All 31 prior defence-header stops advance to their next required operation. Final first-stop
counts are 32 assembly, 28 rune reconstruction, 19 parser, 18 crafted affixes, 15 base buffs
and four unique database; all 116 items remain pending. Armour data is nonempty for 31
items and absent for 85. Caller/imported XML bytes, inventory ownership and every original
build remain unchanged. Adapter/data identities change deliberately; elapsed/startup times
and PoB export child ordering differ. Export normalization is not source preservation or a
claim that ordering is irrelevant. The runner's exit 1 records expected native rejections,
not failed imports or failed reference calculations.

Evidence: `runs/defence-header-data-final.json`, `runs/defence-header-oracle-final.json`,
`runs/defence-header-import-targets.log`, `runs/defence-header-import-clippy.log`,
`runs/defence-header-cli-final.json`, `runs/defence-header-machine-review.md`,
`runs/defence-header-format-final.log`, `runs/defence-header-wasm.log`,
`runs/defence-header-native-dependencies.log`, `runs/defence-header-corpus-final.json`,
`runs/defence-header-corpus/index.json` and `runs/defence-header-docs-audit.json`.
These local ledgers supplement committed reproducible tests.

### Resume after validation

Root owns integration/docs. Main workspace session 37729 and its final changed-source
reconciliation are complete; no main validation process remains live. parser_data owns
repair CI run 34381171078 and preceding run 34379251966. Keep successful targeted results;
do not restart broad suites merely for bookkeeping.
The defence-header worktree contains the complete next slice, ready for integration once the
preceding gates are resolved. Preserve both documentation histories when reconciling updates.

Next item work can implement original base flask/charm buff loading through the existing
parser provider. Read-only inventory `runs/defence-header-next-base-buffs.json` finds 13 charm
bases with buffs and no current flask buff definitions; this is not a supported-base list.
Both independent source branches, per-ParseRaw duplicate suppression, repeated base selection,
parser error ordering and assembly-mutated line payloads need explicit coverage. Catalogue
records remain injected. Stateful parser/callback execution and complete assembly remain
separate pending work; no broader build admission follows from this loading step.

## Base flask/charm buffs checkpoint

Implemented and locally validated in `C:/code/poe-optimizer/runs/base-buffs-worktree`,
branch `codex/base-buffs`, based on defence checkpoint `89af004` and the shared fixture
repair `e602ba3`. Main's parser implementation and the preceding defence worktree remain
separate. The new source clone is clean at the existing pin. No data migration or general
actor/action/candidate model change is included; the B3 decision request remains pending.

Selected bases invoke the existing native parser directly for each injected buff string,
including duplicates and empty strings. Flask/charm initialization and once-only authored
suppression are independent and ordered. Present-empty tables prevent regeneration within
a parse; reparses reset both rows and suppression sets. Sparse metadata follows first-gap
`ipairs` traversal. Generated rows start with absent range/scalar/selection values. XML
ModRange visits them before ordinary rows; provider errors and unavailable operations
preserve only completed rows. Source probes distinguish malformed non-string attempts from
the string-only native trace without inventing values or requests. The worktree extends
[the item-loading contract](item-source-and-loading.md) with the complete behavior.

The parser request now charges text before cloning it. Independent review corrected its
initial oversized-example estimate: metadata strings are capped at 4 KiB, so this closes a
bounded budget-accounting gap rather than a large-allocation bypass. Limits and source/provider
call ordering are unchanged. Generated rows have a separate count bound even for short
authored inputs. Immutable catalogs remain shared; no Lua runtime or subprocess enters
native loading.

| Gate | Result |
| --- | --- |
| Import/provider contracts | 45 pass: 31 loading contracts including six new buff/resource cases, nine formatter and five parser-provider regressions; strict affected-target Clippy passes |
| New independent source tests | Nine have passing current coverage: shipped native parser states, independent family/duplicate/empty cases, sparse/false definitions, base variants, reparses, genuine parser errors/deferrals, real assembly and XML ranges |
| Existing source regressions | 19 pass: 12 item-loading and seven provider tests; all 116 corpus prefixes match, including 257 parser requests, 216 formats and 36 complete preassembly states |
| Final source/code identity | Two native-source regressions rerun after charge-before-clone change; strict affected-target Clippy passes. The ledger retains the earlier fixture failures and passing replacements without claiming a failed full attempt exited successfully |
| CLI | All ten native-only inspection tests pass; the new injected-buff contract also passes with PoB enabled. Strict CLI Clippy passes in both feature configurations |
| Portable/static checks | Five WASM libraries compile; native runtime dependencies exclude PoB/Lua; workspace formatting and documentation link checks pass |
| Fresh caller corpus | All five imports, inspections and reference evaluations succeed. All 110 reference measurements, full backend/context/coverage/warnings/raw actors, imported XML and 834 source instructions are preserved. All five native rejection reasons are unchanged |
| Publication | Combined defence/buff implementation `8781c7c959ac548ea74d364f0207cb0e614633dc` is integrated on main with the readiness repair. Preceding branch [CI 34383255010](https://github.com/Azaril/poe-optimizer/actions/runs/34383255010) has passed formatting/lint and is running tests. Integrated main hosted validation remains pending |

Exactly 15 generated buff rows advance the prior 15 buff stops: seven to assembly and eight
to unique lookup. The other 101 complete loading reports remain exact. Final first stops are
39 assembly, 28 rune reconstruction, 19 parser, 18 crafted affixes and 12 unique database;
all 116 items remain pending. Runner exit 1 records the five expected full-native rejections,
not failed source or reference evaluation. Independent complete-build goldens remain fixed.

Frozen production machine SHA-256:
`2e37309bc1cd75d246620e972d17cdee1d0e9b6e92be4c5e2311a80923446650`.
Package bytes retain schema 17 and SHA-256
`220ea3decf5ee887c2b2d7890be253837f21e6c6bf45424e07a85a8573000acf`.
Native CLI SHA-256: `c8108f3b3298f5cd5cbecd6f60c980a5e708573450c212a11a72b04e7dfc586c`;
reference CLI: `b4c8e12e434642b6d66215762f511c9b193803e0bc655f041bc53f4741a36614`.

Evidence: `runs/base-buffs-oracle-final.json`, `runs/base-buffs-import-targets-final.log`,
`runs/base-buffs-import-clippy-final.log`, `runs/base-buffs-resource-contract.log`,
`runs/base-buffs-native-cli-all-inspection.log`, `runs/base-buffs-native-cli-clippy.log`,
`runs/base-buffs-default-cli-contract.log`, `runs/base-buffs-default-cli-clippy.log`,
`runs/base-buffs-wasm.log`, `runs/base-buffs-format-final.log`,
`runs/base-buffs-docs-audit.json`, `runs/base-buffs-resource-review.json`,
`runs/base-buffs-binary-freeze.json`, `runs/base-buffs-corpus-final.json` and
`runs/base-buffs-corpus/index.json`. All local validation sessions are terminal.

### Publication and resume

parser_data monitors combined-branch CI 34383255010 and parser-repair CI 34381171078,
and retains prior-run 34379251966 evidence. Root integrated the item-loading and readiness
checkpoints on main, preserving prior resume edits. Follow the integrated main run when it
starts. Preserve the final binary/data fingerprints and all passing local results;
no broad local suite needs restarting. Keep any new CI failure and its precise repair as
separate evidence.

The completed dependency investigation covers original unique-database construction and
requirement lookup. The catalog contains 443 raw prototypes in 30 groups; raw
base-level projection cannot replace executed-item requirements. Construction can consult
already-built uniques and source iteration order is not guaranteed by sorted catalog keys.
Read `runs/base-buffs-next-unique-audit.md`, `runs/unique-native-seam-audit.md` and
`runs/unique-requirements-data-seam-audit.md` before implementing the next slice. Stateful modifier parsing, callbacks and full assembly remain separate work; do not
silently fill requirements or numerical effects from metadata recognition.

The phase-only runtime probe progressed to a confirmed reference correctness defect;
see the following checkpoint. Unique lookup implementation must use a completed database,
not treat a still-loading absence as a missing unique.

## Unique requirements active checkpoint

Worktree `runs/unique-requirements-worktree`, branch `codex/unique-requirements`, starts
from main `d213a1fc`. This implements the next existing item-loading dependency through
the injected data seam. It does not change the B3 actor/action/candidate model, admit
unique numerical effects, or claim full native item construction/assembly.

The schema-18 package adds a distinct `unique_requirements` section while
retaining all 26 previous section values and digests. A validated complete catalog
contains exact canonical keys, optional finite natural/equipment levels, source-derived
ordered runic-prefix/separator policy, input identities and a disposition for every raw
prototype. Unavailable data cannot produce a definite miss. Constructor failures,
unfinished loading, omitted outcomes, collisions or cross-entry fallback dependencies
must be resolved or explicitly rejected, not converted into absent records. Fully native
prototype construction remains unfinished; these are injected item requirement facts,
not cached candidate evaluations.

The existing snapshot gains `unique_requirements()` with immutable borrowed lookup.
Exact-key lookup precedes one source-defined leading-prefix removal with a nonempty
suffix. The native provider performs no Lua execution, I/O, subprocess startup or key
concatenation allocation. Builtin providers use the selected snapshot and stop explicitly
when its catalog is unavailable. Existing `with_dependencies` composition continues to
forward caller-supplied unique lookup; `with_native_unique_lookup` explicitly combines
selected-data lookup with supplied parsing/assembly dependencies. Native and reference
backends stay independently selectable.

| Owner | Work and evidence required |
| --- | --- |
| parser_data | Data schema/catalog, bounded complete/unavailable validation, source-authenticated full constructor extraction, two exact package/evidence exports, previous-section identity preservation |
| parser_oracle | Every original constructor/insertion/missing-base outcome, potential exact/runic reads and overwrite audit, original and alternate legal traversal observations, complete lookup and requirement differential tests |
| root | Native provider integration, nil/number requirement semantics, provider composition/boundary tests, documentation and final review/integration |
| parser_number | Native-only/default CLI tests, extraction schema assertions, fresh five-build comparison after freeze, published main/preceding CI monitoring and actionable failure logs |

The frozen package has **443 entries / 443 accounted prototypes**, 27 sections and
21,830,730 bytes; SHA-256
`4e1d7ff27c1c079ed7ef49690a3192cf9c0b36ac7e24539fe56f6cbb6cd70c1e`.
All 26 previous section values/digests remain exact. Construction provenance records 115
source files; the complete extraction allowlist contains 129. Original stored modifier
cache mode is explicit. See [the end-state contract](unique-requirements.md).

Seven observed original constructor passes, spanning traversal and cache changes, agree
bit-for-bit on all 443 entries. An unobserved original control also agrees. The pinned
inventory has no overwrites, missing-base omissions or cross-entry reads. Deliberate
collision, runic dependency and missing-base witnesses verify why the model rejects or
accounts for those cases; this evidence does not certify arbitrary future input order.

Two native numeric defects were reproduced before repair: explicit Lua nil blocked the
natural-to-current requirement fallback, and Rust maximum/default-rune handling lost
source operand order and signed-zero behavior. Native loading now uses the actual rune
field and preserves state assigned before a missing-rune source error. Portable catalog
numbers remain finite; explicit-provider tests also cover non-finite source semantics.

| Validation at this checkpoint | Result |
| --- | --- |
| Data regression | 118 unique tests pass after reconciling two test-only migration repairs; includes 9 new complete/unavailable, stale dependency and full accounting contracts |
| Original exporter tests | 4 passed; strict source number types, all constructed records, failed loading marker and source-derived policy |
| Independent original-source/native target | 6 passed, with one intentional child entry exercised by parents; 443 entries, 1,773 shipped and 15 injected lookups, 14 consumer numeric witnesses |
| Existing affected source targets | 21 passed across base buffs, defence headers and modifier-provider tests; all 12 former unique-database stops advance |
| Native import | All 237 tests passed; focused loading targets include 49 tests covering the numeric repairs and explicit-provider composition |
| CLI | Native 18 and default 14 passed, including two fresh exact public exports; native/default strict Clippy passed |
| Portable libraries | Core, data, engine, import and native compile for `wasm32-unknown-unknown`; browser execution/performance remains untested |
| Strict checks | Data/import all-target, PoB library and four affected PoB test targets pass strict Clippy; workspace formatting and Git whitespace checks pass |
| Engine data injection | 13 passed; selected numerical records still reach prepared calculations |
| Fresh five-build corpus | All 110 reference measurements exact; 104 complete loading reports unchanged and 12 unique-database stops advance to assembly; all 834 source instructions preserved. All five native rejection messages remain exact |

Current evidence is under `runs/unique-requirements-*`, including section-preservation,
data-source-freeze, source-order-audit, oracle-final, root-freeze, binary-freeze,
corpus-final, data-final, cli-validation-final, production-review, import tests/Clippy,
WASM and CLI logs. All 152 recorded final
production/manifest fingerprints match the binaries used for the corpus. Original
fail-before logs remain alongside the passing regressions. Preserve caller input,
independent goldens, the source pin and all prior frozen binaries/evidence.

All local validation sessions are terminal. The independent production review found no
actionable correctness issues in the catalog, exporter or provider boundary. Two original
migration-test failures remain recorded: a tree mutation now first hits the derived-input
check, and a skill-catalog fixture still asserted schema 17. The repaired tree fixture
explicitly makes unique requirements unavailable to preserve its independent structural-pin
assertion; the other fixture now asserts schema 18. Changed cases and all unreached tests
pass. These repairs do not change production behavior or the frozen validation binaries.

Resume: verify the publication identity and hosted CI outcome, then continue the authored
affix slice below. Stateful modifier parsing, callbacks, crafted-stat generation, runes and
complete item assembly remain separate native work. The full native-parity and broad
optimizer goal remains incomplete.

### CI repair and provenance follow-up

The first published unique-requirements run failed `clippy::needless_borrow` at
`unique_requirements_extract.rs` lines 572 and 603. Only those two test calls change from
`&tree` to `tree`; the reviewed package and production operations remain exact. Retain
`runs/unique-requirements-ci-job-102599887273-annotations.json` and
`runs/unique-requirements-ci-job-102599887580-annotations.json` as the actual CI evidence.
The final `cargo clippy --workspace --all-targets --locked -- -D warnings` gate passes,
including library-test targets. All four original-source tests and all three public CLI
extraction tests pass again. Formatting, Git whitespace and all 47 documents / 508 local
links pass. Evidence is in `runs/unique-requirements-ci-workspace-clippy.log`,
`runs/unique-requirements-ci-source-edit.json`, `runs/unique-requirements-ci-source-tests.log`,
`runs/unique-requirements-ci-extraction-cli.log` and the repair publication ledger.
The hosted repair run remains a separate gate.

The exporter fingerprints its complete source file, including tests, so this repair
changes its implementation identity even though generated package bytes are unchanged.
Keep the original corpus/binaries frozen; fresh public extraction must still reproduce
the reviewed package. Do not relabel the original binary evidence as a post-repair run.

A separate provenance review confirmed that CLI/corpus envelopes already retain the
full data-code fingerprint, including the new lookup implementation. A host using only
the standalone item-loading report should additionally retain
`poe_optimizer_data::implementation_fingerprint()`. The [library report contract](item-source-and-loading.md)
now states that distinction; no numerical code or report schema change is involved.

## One-argument number factory checkpoint

Code: `8b679da344ce56519a605631ad9f0bb0ce7f255b`. Package schema 25/parser 6;
semantics `poe2-native-profiles-v25`; 23,177,945 bytes, 27 sections, 129 authenticated
sources; SHA-256 `334b440b6a567bf75fd265322925d100a86161fa615f41cb0e4c5c70e3308321`.
The PoB source remains pinned at `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`.

The extractor authenticates the actual original global `tonumber` C function before
source construction and verifies the raw slot, pointer and kind afterward. Lua owner
environments retain their independent original-global proof. mlua exposes no Lua
environment for the C function; no fictitious environment or helper/capture ID is
recorded. Whole-body lowering accepts exactly one explicit argument and rejects
shadowed, omitted, base and extra-argument forms. Factory roots still require a table
or nil. No new game-literal policy or generic function-call language is added.

The native operation evaluates its child, charges string bytes and scalar output,
then reuses the existing byte scanner. Numbers preserve their bits; non-convertible
represented values return nil without invocation or stringification. Child errors and
argument order remain observable. Immutable catalogs support independently injected
expressions on concurrent callers.

| Scoped gate | Result |
|---|---|
| Engine library/contracts/data injection | 62 passed |
| Affected import loading/formatting/parser/runes | 62 passed |
| Data library/integration | All 151 passed |
| Extraction/authentication | 26 passed |
| Original-source integration | 100 passed |
| Native/default CLI observations | 46 passed |
| Workspace/native strict all-target lint, formatting and dependency audit | Passed |
| Five WebAssembly libraries | Passed |
| Two independent public exports | Byte-exact package reproduction |

These are scoped gates, not a full workspace or full import-suite test claim. The nine
new source tests cover 81 real bodies/103 natural call sites; 721 aliases (709 exact
graphs and 12 matching source errors); 20 nested shapes (127 graphs and 33 errors);
605 byte cases; 11 direct IEEE pairs; cache/provider/metamethod controls; 10,368 direct
original-body calls and 4,840 changing-input warmed conversions. The actual `tonumber`
function pointer appears in completed live traces. Each case is measured before a
subsequent flush; recording or cache hits alone are not execution evidence.

The expanded ordinary suite covers 143 bodies, 282 natural output pairs and 18,304
warmed calls. Shared-loop trace pressure required an observer repair to measure completed
live traces per case. The original attempts and final passing reruns are preserved.
The initial CLI failure was a test-only expectation for the existing non-finite metadata
message; production behavior was unchanged. Source witness/trace observer corrections
are also retained in the oracle ledger. No production discrepancy was found.

The broad Special generator has two preceding gaps: one quality pattern is shadowed by
a longer conditional rule, and the dagger suppression witness drops a literal plus.
The newly admitted equipped-item suppression callback exposes the same old generator
defect, but its dedicated new witness proves its natural call. These three observations
must not be relabeled as three unsupported mechanics or confused with the separate seven
full-item source observation gaps. See `runs/number-factories-selection-gaps.json`.

All 116 complete corpus item reports remain unchanged, as do 110 reference number bits,
834 source instructions and five native rejection messages. The source join covers
116 native states, 109 complete original states (82 preassembly and 27 parser-frontier),
112 affix states, 598 parser prefixes and 496 formatter prefixes. Seven full-state and
four affix observation gaps remain explicit. No new complete item or build support is
claimed by this phase.

The reconciled ledger is `runs/number-factories-validation-final.json`. Component records
include root/data validation, source oracle, corpus comparison and source join. The
initial 214-input CLI manifest is preserved; the final 215-input manifest adds the
explicitly repaired external ordinary warm helper. CLI production inputs and immutable
binaries stayed exact. Preservation checks all 44 protected files, both caller inputs
and the preceding 514-file Flag-worktree freeze. The next complete common-assembly scope
and pending design choices are documented in the resume point above.

## Closed Flag factory checkpoint

Code: `e862651c0143ad1973b9cd50f1583d90059d47be`. All scoped local gates pass.
Package schema 24/parser 5; semantics `poe2-native-profiles-v24`;
23,085,269 bytes, 27 sections, 129 authenticated sources; SHA-256
`dd7f7f2dd75ae5655c830a41a783f37674ea4a1b5f11b5cbbc2a5708725e721d`.
The PoB source remains pinned at `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`.

The complete source verifier admits 68 additional Special bodies. Their pre-anchor
aliases are not another 68 bodies. Every owner captures the authenticated Flag helper,
which captures the authenticated constructor. Twenty-three owners lack a direct
constructor capture; their provenance retains that distinction. The extractor verifies
the complete wrapper, actual function bindings and literal prefix types. Data validation
checks the captured path and source anchors; no production callback-ID whitelist is used.

The engine evaluates arguments in order, binds the first value or nil as the name,
inserts the injected string/boolean prefix and forwards every remaining position,
including nil holes. It reuses constructor logic without a second argument vector.
Immutable definitions remain shareable across workers, with request work/output bounds.
An independently injected empty/NUL/UTF-8 prefix and false value are supported. The
existing opaque function-value boundary remains: the preserved isolated constructor
has a different lexical capture graph from the full module, including when nested
inside a returned Flag helper. Operational table parity does not erase that difference.

| Scoped gate | Result |
|---|---|
| Engine library/contracts/data injection | 57 passed |
| Affected import loading/formatting/parser/runes | 62 passed |
| Data library/integration | All 146 passed |
| Extraction/authentication | 21 passed |
| Original-source integration | 91 passed |
| Native/default CLI observations | 44 passed |
| Workspace/native strict all-target lint, formatting and dependency audit | Passed |
| Five WebAssembly libraries | Passed |
| Two independent public exports | Byte-exact package reproduction |

These are scoped gates, not a full workspace or full import-suite test claim.

The new source matrix covers every admitted Flag selection, 408 alias graphs,
constructor forwarding, source errors, arbitrary injected prefixes, nested calls,
copy behavior and 8,704 live warmed calls. Existing factory/string/caller/item/session/rune
regressions remain passing. A stale negative extraction fixture and a nested formatting
lint in a source-test helper were corrected; original failed attempts are preserved.
The corrected source target passed again. Only one external test helper changed after
the CLI freeze; its generated Lua text and all production/embedded inputs stayed exact.

The final corpus preserves 110 reference values bit-for-bit and all 834 source instructions.
One item advances from parsing to assembly; 115 complete reports remain identical.
The strict join covers all 116 native states, 109 complete original states (82 preassembly
and 27 parser-frontier), 112 affix states, 598 parser prefixes and 496 formatter prefixes.
Seven full-source observation gaps and four affix gaps remain explicit. No new whole
build or complete item-assembly support is claimed.

The reconciled ledger is `runs/flag-factories-validation-final.json`. Component
evidence uses `runs/flag-factories-*`: root/data validation, preservation,
source oracle, CLI validation, corpus comparison and source join. The original 209-input
CLI manifest is retained; the final current manifest reconciles the one test-only edit.
Preservation verifies 44 protected files, both caller inputs and the preceding
507-file string-worktree freeze; publication checks the same content in main. The reviewed next numeric slice and pending conditional
proposal are described in the resume point above; neither is implemented by this phase.

## Bounded string factory checkpoint

Code: `11e6f3906b30f5adba8e0847e6de6b84ea0f6894`. Package schema23/parser4;
PoB remains pinned at `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`. This extends
the injected factory expression seam with Concat and the one closed FirstToUpper
operation. It does not admit arbitrary helpers, callable tables, new build profiles
or completed numerical mechanics merely because their text now parses.

The extractor proves the complete three-line helper, its original global environment,
raw string primitives and string metatable lookup, then consumes each owning callback's
whole source body. It verifies the owning callback's captured helper identity and
retains existing closure/table graphs. The new authored pattern is compiled once with
the immutable catalog; selected native calls use per-request matching/output budgets.
The lowerer preserves unary/concat precedence and parentheses and accepts exactly one
helper argument. Complete unsupported bodies remain opaque even after their first
newly recognized expression.

Source parity covers all 38 new original bodies at 48 real selections (28 Special,
10 first-tag and 10 second-tag). The additional alias matrix has 181 exact graphs and
nine ordered source errors; concat/grouping has 104 graphs and 116 errors; configured
helper patterns have 112 graphs and 38 errors. All 256 byte values are compared
through original/native public outputs. The 127 non-NUL ASCII policy fields stay
within the existing catalog text boundary, while raw captures and AST text retain
NUL coverage. Direct original warm execution observes all 38 factory prototypes and
the helper in live traces over 4,864 calls.

Existing ordinary coverage expands to all 120 callbacks, 237 real public selections
and 15,360 direct warm calls. Existing Special coverage retains whole-body graph and
public-copy checks. Total source integration scope is 84 tests, zero failures/ignored.
A callable table method stops natively at `firstToUpper receiver method` with its
actual callback ID; this boundary is not mislabeled as a successful original function
execution. The constructor's pre-existing isolated/full-module lexical graph difference
also remains explicit.

Preserved failed observations: the first source matrix passed five tests and exposed
two fixture mistakes. A malformed custom helper pattern was applied before dictionary
construction, where original source correctly errored; the corrected runtime control
constructs the original dictionaries first and changes only the authenticated helper
literal for selected calls. A policy-text fixture incorrectly contained NUL; corrected
policy fields respect the schema while raw-byte/literal controls preserve NUL coverage.
The final full seven-test matrix passes. Early compilation failures during coordinated
AST/test edits are retained in component logs; final workspace and native strict lint pass.

The final corpus has 48 generated rows, 58 generated calls and 57 origins. Exactly two
formerly deferred leech items reach Assembly; 114 complete reports and all reference
number bits remain unchanged. Native build failures stay byte-identical to baseline,
and the corpus runner's nonzero exit is attributed only after independently checking
all five successful imports, inspections and reference evaluations. No changed rejection
is silently treated as an expected limitation.

The next read-only Flag audit covers the complete catalog. Sixty-eight new syntax
candidates include 23 whose owning callback has no direct mod capture. A future closed
node must preserve flag-to-constructor provenance and injected type/value literals;
it cannot invent a direct constructor capture or become a general function executor.

## Ordinary factory invocation checkpoint

This bounded phase enables the existing three Pure Prefix and 107 Pure ModTag recipes
at their actual ordinary-parser call sites. It adds no recipe expression kinds, general
helper interpreter, item assembly or actor/action/B3 migration. The schema22/parser3
package adds injected `tag_capture_numeric_pattern` policy and source spans proving the
three invocation branches. All 26 unrelated sections, existing graph/recipes and
constructor descriptors remain exact. Source pin and caller inputs remain fixed.
The installed package has 22,969,857 bytes
and SHA-256 `7921802667c06905e11dfb8f68fd4a7b8e877df0a1e4132c4aacba77b48cb8bb`.
All 26 other sections and the old parser projection are exact; the three new invocation
spans cover original lines 6653–6657, 6671–6679 and 6680–6689.
Baseline: `runs/ordinary-factories-baseline.json` and
`runs/ordinary-factories-original-game-data.json`.

Prefix passes only raw captures. Both tag positions first run the original method-call
precheck with the injected unanchored pattern (`%d+` in the original source). A match
causes leading numeric conversion; otherwise the leading value is raw cap1. All original
captures follow, duplicating cap1 and retaining nil positions. Missing or position cap1
fails method lookup before body entry, including for Unsupported callbacks. Malformed
patterns remain lazy source errors; unrelated Special, Prefix and static metadata rows
do not execute the tag precheck. Only a truthy first tag result enables the second scan,
which uses a fresh capture vector. Existing contribution, wrapper and public-copy order
is retained.

The native engine now uses a borrowed Raw/Leading argument view, shared immutable
compiled policy and per-request resource budgets. No invocation argument vector or raw
leading-string copy is needed. Engine `cargo check` passes. Seven new engine contracts
cover raw/converted captures, injected patterns, method-error precedence, skipped second
tags, concurrent separate catalogs, mandatory precheck work and output exhaustion before
a later body error. These are authored external-catalog tests, not production whitelists.
CLI coverage adds caller-authored metadata and pending/error cases; its targeted native
case passes. All seven new engine contracts and the existing 11 library, five Special
factory and nine parser contracts pass (32 checks). Strict engine lint and five portable
WebAssembly libraries pass. The first source gate compares all 217 actual ordinary
outputs, 49 injected argument graphs and four matching precheck errors. The seven new
source tests also pass: 95 configured-pattern graphs, 38 ordered errors,
60 combined prefix/form/two-tag cases and all 110 closures warmed through 14,080 direct
calls with 110 live source lines. All 77 source integration checks pass (seven new and
70 affected existing checks), with no ignored tests. The final corpus/source join passes. The 13 engine data-injection checks bring the engine total to 45.
All 62 affected import checks pass: 37 loading, nine formatting, five parser and 11 rune.
An old PerStat-tag deferral expectation was replaced with exact newly supported metadata
assertions; the failed original log and no-execution target-name typo are preserved in
`runs/ordinary-factories-root-validation.json`. No production correction was needed.

Read-only original-source probes select all 110 real Pure ordinary callbacks, with
three Prefix, 107 first-tag and 107 second-tag observations. These confirm actual call
sites and error/argument behavior; they are preparation evidence, separate from the
completed Rust full-output parity gate. Plans and probes are in
`runs/callback-factories-next-native-plan.md/json` and
`runs/ordinary-factories-oracle-plan.md/json`. Complete source tests must exercise both
tag positions, empty/nil truthiness, byte/position captures, public retry/copy behavior
and cold/warm calls.

The fresh five-build corpus retains all 116 item reports and current first stops:
82 Assembly, 30 ModifierParser, three RuneReconstruction and one AdvancedCopyAffixes.
All 30 parser stops select seven Unsupported Special shapes; this phase adds breadth
independently of those examples. Fresh frozen binaries retain all 110 reference number
bits, 834 source instructions and five rejection messages. The final source-state join
pairs all 116 native states, 109 full original states, 112 affix states, 589 parser and
491 formatter prefixes, plus 44 generated rows/54 calls/53 origins. All 40 default/native
CLI observations and both byte-identical public package exports pass. Evidence is in
`runs/ordinary-factories-cli-validation-final.json`,
`runs/ordinary-factories-corpus-final.json`,
`runs/ordinary-factories-source-corpus-join.json` and
`runs/ordinary-factories-oracle-final.json`.
Keep the seven prior full-state observation gaps and opaque constructor-value limitation
explicit. Whole-native admission remains Spark/Mace.

All 137 data tests and nine extractor/authentication tests pass. Final data evidence is
`runs/ordinary-factories-data-validation.json`; root scoped gates are in
`runs/ordinary-factories-root-validation.json`. The final combined ledger is
`runs/ordinary-factories-validation-final.json`. It retains the obsolete test expectation,
initial target-name typo, transient unused test imports and narrowly corrected comparator
allowance for the three planned provenance spans. No production repair was required.

Published as `35960b6403c52be4a9ab2aed60c43fd3205ffe27` on `origin/main`; preservation
evidence is in `runs/ordinary-factories-publication.json`.

Resume: continue the reviewed 38-candidate string-factory slice in a new worktree from
current main. Keep this phase and its binaries frozen. The next plan and independent
review are `runs/ordinary-factories-next-native-plan.md/json` and
`runs/string-factories-oracle-plan.md`. No broader helper, branching or B3 approval is
implied by the bounded extension. Workspace
strict lint and formatting already pass on the frozen sources. Reuse terminal gate evidence instead of repeatedly running long
package-validation targets without a source change. Record exact tests, failures and
replacements, binary/source hashes and publication before starting another phase.

## Pure special callback factories checkpoint

This phase continues the agreed native parser and injected-data seams. It does not
implement the unapproved B3 actor/action/candidate proposal, general Lua execution,
helper control flow, mutable callback environments or complete item assembly.
Work is isolated at `runs/callback-factories-worktree`, based on `4396725`; the source
clone remains pinned and clean. Baseline: `runs/callback-factories-baseline.json` and
`runs/callback-factories-original-game-data.json`. The rune worktree and final binaries
remain frozen.

The admitted grammar has fixed parameters and one return expression: scalar literals,
arguments, captured scalars, injected policy-table fields, negation, ordered table
construction and the authenticated `createMod` constructor. Constants, record names and
keys remain injected data. The schema21/parser2 migration adds one lowering disposition
per closure while retaining the original graph, policy, declarations and dependencies.
All 26 other package sections are exact. The package contains 22,969,307 bytes with
SHA-256 `a043ddacb6ae232999e459cbb6ed677995969065e7449e7da9429efa1b0f4953`.
There are 1,035 Pure recipes and 616 Unsupported dispositions. Only the Special call site
executes recipes in that checkpoint: 925 memberships. The 107 ModTag and three Prefix recipes retain explicit
pending real call sites. Lowering proves complete source shapes and captured bindings,
without selecting callback IDs or build names.

Native Special invocation passes `tonumber(cap[1])` followed by all raw captures,
preserving nil holes and numeric position captures. Nil is a successful absent modifier
list, distinct from an empty table. The callback returns directly before the existing
final public-parser copy. Constructor arguments retain their types, source/flags/keyword
positions, sparse tags and shared tables. Work, output and recursion limits are charged
before expanding allocations. Immutable catalogs are shareable between concurrent
native requests; no runtime Lua, subprocess or external lookup is introduced.

Current scoped gates include 38 distinct engine checks (11 library, 13 injection, nine
existing parser contracts and five factory contracts), all 252 import tests, 38 CLI
observations, native-only strict lint/dependency checks and five portable WebAssembly
library checks. Two fresh public exports reproduce schema21 byte for byte. All 135 data
tests, 105 source integration tests and six source extraction/authentication tests pass.
One direct-entry source worker is ignored and parent-exercised. Final workspace strict
lint and formatting pass; no current failure remains in the scoped gates.
The engine logs are `runs/callback-factories-engine-regression1.log`,
`runs/callback-factories-engine-contract1.log` (nine retained existing contracts) and
`runs/callback-factories-engine-contract2.log` (five final factory contracts).
Other completed gate evidence: `runs/callback-factories-import-suite1.log`,
`runs/callback-factories-wasm-final.log` and
`runs/callback-factories-cli-validation-final.json`.

The cold source enumeration compares all 1,035 Pure bodies through labelled Special
aliases: 6,021 exact return graphs and 189 matching source errors. This is a body proof,
not admission of the 110 ordinary call sites. Real Special-pattern witnesses select and
compare 923 distinct factories; two other selections are explicitly unpaired. Separate
warm controls force 768 direct calls across three authentic recipes, covering all 12
expression operations present in real Special recipes. Trace evidence includes live
factory and original constructor lines. Original constructor, dispatcher, public-copy,
source-error and injected-data controls remain separate observations.

A deliberate opaque-value probe exposed a preexisting observation limitation: the
catalog's unchanged standalone constructor descriptor has no Lua upvalues, while the
full original module captures local `select` and `type`. Scalar/table operation parity
remains exact and the functions' operation identity is proved, but returned constructor
closure graphs differ. The original failed comparison is preserved. A dedicated control
asserts the difference and proves that both function-valued outputs defer at the finite
item-metadata adapter. No graph normalization, package rewrite or function-valued
modifier support is claimed. Future support must address lexical environment identity.

The fresh five-build corpus retains all 110 reference number bits, 834 instructions and
five whole-native rejection messages. Ninety-three complete item reports are unchanged.
Twenty-three parser stops progress: 15 to assembly, eight to later parser callbacks.
Current first stops are 82 Assembly, 30 ModifierParser, three RuneReconstruction and one
AdvancedCopyAffixes. All 116 final CLI-native states join to the immutable source ledger;
109 original full states are paired (79 preassembly and 30 parser frontier), including
all 23 progressed items. The join checks 112 affix states, 589 parser and 491 formatter
prefixes. Seven prior observation gaps remain: three jewel-radius, three rune-ambiguity
and one advanced-ordering state. The final records are
`runs/callback-factories-corpus-final.json`,
`runs/callback-factories-source-corpus-join.json` and
`runs/callback-factories-final-source-corpus.json`.

All 161 build inputs, 17 data/extractor files, frozen binary records, 44 protected files
and both caller inputs retain their recorded raw hashes. The complete original source
clone remains clean at the pinned revision. Final evidence is consolidated in
`runs/callback-factories-validation-final.json`, referencing data, source and CLI ledgers;
`runs/callback-factories-root-freeze-final.json` records the final file identities.
Failed early fixtures, lint attempts, build preflight and the deliberate opaque-graph
comparison remain preserved with their scoped replacements.

Published code: `27c11a2b253eacf2a21e9cd9c6905b07090c6013` on `origin/main`. The
publication ledger is `runs/callback-factories-publication.json`. All 496 frozen files
retain raw hashes; all 496 main files have matching normalized content. Ninety-two raw
cross-worktree differences are line endings only, including the 14 preexisting protected
file differences. Both caller files remain exact. Hosted code/docs CI is tracked
separately from local gates.

Resume: inspect exact-head CI, then continue pure Prefix/ModTag invocation in an isolated
worktree. Do not modify the frozen callback binaries or source after publication.
The next audited slice is pure Prefix/ModTag invocation; see
`runs/callback-factories-next-native-plan.md/json`. It changes no expression grammar,
but requires an injected numeric-precheck pattern, exact argument conventions and
source validation of both ordinary tag positions. It is expected to remove none of the
current 30 Special first stops. No parser success implicitly certifies a numerical
mechanic or broader whole-build support.

## Socketed-augment loading checkpoint

Implementation is in `runs/rune-loading-worktree`, branch `codex/rune-loading`, based on
`1d742b8d7c14ea2f9a76ce5bf70b4cc2640bfb00`. See the
[state contract](item-source-and-loading.md#socketed-augment-state) and
[data seam](native-data.md#schema-migration). The complete native evaluator and broad
joint optimizer remain unfinished; this phase does not change Spark/Mace admission.

Schema **20** / `poe2-native-profiles-v20`, item-loading schema **4**, contains
21,836,454 bytes with digest
`a3c7515db5afb650bb1d1618c8ca85a5db92f16eee0cfaf73d51f1ed67e5aea9`.
All 25 unrelated sections, existing affix data, all raw rune records, the pinned tree and
all 443 unique requirement facts are unchanged. Original constructors regenerate the
unique input binding to the new item catalog; two fresh public exports reproduce the
reviewed package exactly. The source pin remains `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`.

Implemented operations include injected rune/socket headers, variant-selected context
hints, Bonded raw-line skipping, source slot classification, ordered authored rebuilds,
number-text stacking, disabled-template restoration, normal/Bonded annotation searches
and all-slot level requirements. Generated parser calls and rows carry explicit socket,
slot and definition origins; source line indices are absent for generated text. Native
numeric and loading work uses no Lua or subprocesses. Stateful assembly and unsupported
parser callbacks still stop explicitly.

The annotation proof checks complete minimum count vectors and source-order effects.
It does not choose a sorted winner for equally good identities. Equal vectors with
implicit trailing components can still produce ambiguous types under Bonded caps;
original opposite dictionary orders demonstrate that boundary. Group storage, sorting,
patterns and recursive-search equivalents all have independent work limits. Source
errors retain their partial state; exhausted native resources are not no-solution results.

| Gate | Evidence and result |
| --- | --- |
| Data and extraction | 129 data tests, two policy/source tests and one complete original catalog test pass. `runs/rune-loading-data-final.json` records preservation, failed setup attempts and the final catalog fingerprint correction. |
| Numeric helpers | 14 tests pass, including original text combination/search callbacks, injected grammar, 10,020 number-order renderings and independent ambiguity enumeration. `runs/rune-numeric-final.json`. |
| Import contracts | 252 reconciled tests: 159 unaffected tests from the broad run, 82 final existing loading/source targets, and 11 final rune boundary tests. Retain original failures from the synthetic catalog migration and the old unconditional rune-stop assertion; their replacements pass. |
| Engine injection | All 13 existing data-injection tests pass; `runs/rune-loading-engine-injection-final.log`. |
| Portable/native gates | Workspace strict lint, native-only strict lint, five WebAssembly libraries, formatting and the normal native dependency graph pass. The native graph has no PoB, mlua or LuaJIT dependency. |
| CLI | 36 reconciled observations: final binaries pass 20 native and 13 reference-build inspection tests; three unchanged extraction tests independently produced two identical exports. `runs/rune-loading-final2-cli-validation-final.json`. |
| Final corpus | All five imports, inspections and reference evaluations pass; all 110 reference measurements, all 834 instructions and all five whole-native rejection messages remain exact. 77 complete item reports are unchanged. `runs/rune-loading-final2-corpus-final.json`. All 116 final CLI native states join the frozen source ledger exactly; `runs/rune-loading-final2-source-corpus-join.json`. |
| Original source | 81 source integration tests pass: 14 new rune cases and 67 existing cases. One ignored direct-entry worker is exercised by parents. The final observer pairs 109 full states (64 before assembly, 45 before unavailable parser calls), 511 parser calls, 424 formatter calls and 112 full affix tables, with zero mismatches. `runs/rune-loading-oracle-final.json`. |

The independent corpus frontier is **67 assembly, 45 modifier parser, three rune
reconstruction and one advanced-copy ordering dependency**. Every one of the 38 former
rune stops executes additional native operations. Three reach later rune ambiguities;
35 reach their next non-rune dependency. One former assembly stop now correctly stops
before unimplemented advanced-copy unique ordering. This correction must not be described
as new assembly coverage. The three preexisting jewel-radius stops, the advanced-copy
stop and three rune stops remain outside full-state source pairing.

Late review corrected padded-vector tie checks, bounded temporary grouping storage and
sorting, Lua string-slot indexing, and an ignored injected copy-mode selection. Eleven
native boundary tests cover these cases and resource/error distinctions. The original
binaries, logs, corpus and source ledger remain under `runs/rune-loading-*`; the final
code uses distinct `runs/rune-loading-final2-*` evidence. Never relabel earlier binaries
as having incorporated those fixes. Caller files and all original goldens remain intact.

Next resume work: inspect exact-code hosted CI (publication is complete), then use the
read-only `runs/rune-loading-next-native-plan.md/json` audit to select a coherent
modifier-callback or item-assembly phase in a fresh worktree. All 45 remaining parser
stops currently reach special callbacks; their lowering must preserve injected data and
actual-source parity. The general B3 actor/action/candidate design still awaits user
input and is not implicitly approved by these item-loading phases.

## Authored affix loading active checkpoint

Worktree: `runs/affix-loading-worktree`, branch `codex/affix-loading`, based on published
main `25b8b93db260a93a638b0808831fe84f5bbcc62a`. Local validation is complete; retain this worktree as the frozen checkpoint.

The schema-19 package is generated from the unchanged upstream pin. Its digest is
`dcd0b2fd14a8271f710188312fe5664e39b1edac8f06aa078ab2911b840e79ca` (21,833,240 bytes),
with 27 sections and 129 source evidence files. All 25 unrelated sections remain exact;
`item_loading` adds policy and three source spans, and `unique_requirements` updates only
its item-loading input binding, retaining all 443 constructor facts. The preservation
ledger is `runs/affix-loading-section-preservation.json`.

The native state machine now loads ordered prefix/suffix rows with scalar or independent
ranges, fractured markers and optional limits. It applies postparse limit changes in source
branch order and reconciles active slots against the selected immutable catalog. Reparse
resets both lists but preserves the selected family according to the source lifecycle.
Legacy collisions and unrepresented traversal remain explicit pending dependencies.
The [state contract](item-source-and-loading.md#authored-affix-state) describes these semantics.

`cargo check -p poe-optimizer-import --locked` passes. The first 33-test contract run had
32 passes and one stale expectation that a supported prefix-limit line still stopped at
`CraftedAffixes`. That assertion has been replaced with checks of the resulting limit,
retained line and parser request; the unreached rune/magnitude controls remain. The full
237-test import run passed; a final 37-test contract replaces its 33-test contract and adds
resource/collision and lazy-pattern/preflight checks, yielding 241 distinct reconciled import tests. All 13 engine
injection tests, 34 CLI observations across both feature configurations, workspace and
native-only strict lint, and five WebAssembly library checks pass. The native dependency
tree contains no PoB/Lua runtime. The 124 reconciled data tests, 67 source integration tests and all 34 final2 CLI
observations pass after the last guard-error classification correction. The final2 corpus
comparison and original-source affix join also pass.
Original source-only controls confirm ordinary XML loading invokes `Craft` zero times,
including instrumented and uninstrumented execution. No full native-build capability or
full workspace test pass is claimed.

The repaired published CI run `34392127898` passed lint on Windows and Linux and is testing
as of 2026-09-09 19:24 UTC. The older `34379251966` Windows test failure identifies the
allocation-bound extraction fixture already repaired by ancestor `765ae933`; it is not a
new failure of current code. parser_number retains exact-run annotations and monitoring.

The frozen CLI corpus comparison passes: all 110 reference measurements remain bit exact,
all 834 consumed instructions and source identities are preserved, and all five full native
rejections remain byte-identical. The 98 previously unrelated item reports match completely
after explicitly verifying the new empty affix lists. All 18 former affix stops advance:
two to assembly, ten to rune reconstruction and six to modifier parsing. Nine items now
reach original name completion; the comparison verifies that source transformation rather
than ignoring item names. Current totals are **53 assembly, 38 rune reconstruction and
25 modifier-parser dependencies**. See `runs/affix-loading-final2-corpus-final.json` and
`runs/affix-loading-final2-source-corpus-join.json`; the earlier corpus is retained separately.

The reconciled new source target passes all 15 tests, including 18 parameterized original
header primitive cases and a retained-family NoBase/Jewel error-prefix regression. The initial run's four fixture identity/assembly-count corrections
and empty-grammar validation finding are retained in the original log. A final fixture-only
optimization avoids rebuilding the entire package for each header case; the CLI gates
retain full file-loading coverage. The 45 existing source tests and seven provider tests pass. The final current-code corpus
ledger directly compares 113 full affix tables, including all 18 newly progressed cases;
three old jewel-radius stops remain explicitly unobserved. Fifty complete preassembly
states and 290 parser/249 formatter prefixes also match. No complete item assembly or
broader numerical admission is claimed.

The final guard correction reports malformed reserved-header preflight as unsupported
configuration, rather than claiming an original item operation failed. Actual operation
patterns still raise their lazy source errors only when reached. This changes the loader
identity to `5f01bdef81c33856a088149c78c318688e481eac31e0af6ff0423a3cc83e5c31`.
Earlier successful binaries/results remain under their original identities; final2 artifacts
record the refreshed build and corpus. Production review and root evidence are
`runs/affix-loading-production-review-final2.json` and
`runs/affix-loading-root-validation-final2.json`.

Resume: follow the exact publication's hosted CI outcome and start the socketed-augment
phase in a fresh worktree. All local gates are complete. Final evidence is
`runs/affix-loading-data-final.json`, `runs/affix-loading-oracle-final.json`,
`runs/affix-loading-root-validation-final2.json`, `runs/affix-loading-final2-corpus-final.json`
and `runs/affix-loading-final2-source-corpus-join.json` and
`runs/affix-loading-final2-cli-validation-final.json`. The documentation audit checks
47 documents and 513 local links. Do not overwrite earlier artifacts or present scoped
reconciled gates as a single full workspace test run.
Root owns import/docs, parser_data owns data/extraction, parser_oracle owns independent
source gates, and parser_number owns CLI/corpus gates and hosted CI. B3 remains an
unanswered design proposal.

### Next native dependency: authored affix loading

The bounded follow-up audit is `runs/unique-requirements-next-affix-rune-audit.md/json`.
Implement first-class bounded Prefix/Suffix records and final loading reconciliation,
using the existing injected modifier-table selection. Ordinary XML loading does not call
`Craft`; do not generate crafted stats as a side effect of preserving these records.
Reset lifetime, numeric/array ranges, fractured flags, per-list limits, exact IDs and
source error prefixes need independent original-source tests. Source-derived rarity,
jewel and corruption rules belong in the data policy. Legacy label lookup must distinguish
unique matches from selected ambiguity; sorting a map cannot determine a Lua `pairs`
winner. Preserve original item text and caller-selected data.

The corpus contains 18 such items with 104 authored rows (59 `None`, 30 distinct real IDs);
14 also author runes. These are input counts, not promised advances. Compare every new
state field and actual next dependency against original source before updating coverage.
Rune reconstruction follows separately: authored/rebuilt/inferred rows, socket type/order,
bonded contributions, scalars and authentically missing IDs need their own model and
parity work. Neither phase changes the pending general B3 model proposal.

### Next native dependency: socketed-augment reconstruction

Start from the completed affix checkpoint. The remaining 38 rune-loading stops require
ordered display parsing, reconstruction, annotation, scalar handling and requirements.
Use `runs/affix-loading-next-rune-plan.md/json` and the separate numeric audit as preparation;
they are source/catalog audits, not achieved runtime parity or a promise of corpus advances.

The first coherent candidate is the complete authored-known path. Calling `UpdateRunes`
alone is insufficient: original `ParseRaw` still constructs grouped rune templates and
annotates rows through combination matching. Keep ambiguous minimum solutions and
unrepresented source traversal/sort order explicit. Unknown authored identities, including
inactive positions, disable the original rebuild; do not silently filter them away.

Preserve three separate domains: validation scans every authored rune, rebuild uses active
socket positions, and final rune requirements scan every authored rune and every slot
record. The injected catalog contains 287 identities and 594 slot records, including 479
bonded records. Slot classification, identity markers, matching/format patterns, ordering
and tolerance constants belong in a source-derived data policy. Reuse native number,
pattern, parser and formatter seams; prepare immutable context indices once, with bounded
per-item scratch and exact callback evidence.

Before expanding admission, independently prove authored and disabled rows, broad/specific
slot overlap, ordinary/bonded combination, duplicate and equal-order records, unknown and
excess entries, variant/reparse lifetime, requirements, and source-error prefixes. Keep
rune inference without authored IDs and other unavailable paths explicit until implemented.
Measure real corpus advances only after that complete source-order validation. This work
does not implement `Craft`, complete item assembly, or the pending B3 actor/action model.

## Reference item-database readiness checkpoint

Implemented as `45ed37149b48a634613bfaf36b6ac1e656040a84` in
`runs/reference-readiness-worktree`, branch `codex/reference-readiness`, based on `8781c7c`,
and integrated on main. This changes optional PoB reference initialization, not the
native hot loop or build admission. No B3 model change is included.

The ordinary host imported caller items while the upstream `LoadItems` coroutine still
held `uniqueDB.loading = true`. A phase-only probe observed 15, 60 and 113 database entries
after initialization, import and fresh calculation. Lookup tracing and a separate control
that completed the original callback confirmed an observable requirement error: a copy
of corpus line three with only Item 10's authored `LevelReq: 49` removed exported level 40
for Lavianga's Spirits. Completing the database before import exported 49. Recalculating
the prematurely imported item did not repair its cached requirements. The unchanged
production executable reproduced level 40, so the finding does not rely solely on an
instrumented runtime. Caller input and committed fixture/golden bytes are preserved.

The adapter now runs the original `main.onFrameFuncs.LoadItems` callback to completion
before `loadBuildFromXML`, retaining original prototype construction, iteration and parser
cache behavior. Both unique and rare database loading flags must be cleared. Errors and
prompts propagate; a 16,384-callback operational guard supplements the existing supervised
process deadline. Unrelated frame callbacks do not run during this readiness step. The
initialization script is included in the adapter fingerprint; old evidence identities stay
unchanged. This also prevents rare-prototype parser initialization from interleaving with
caller import.

The independent bundled-DLL calibration harnesses now finish the same upstream loading
task before fixture import, using their own bootstrap code. Fresh outputs retain their
new harness provenance; the six original committed goldens remain unchanged.

| Gate | Current result |
| --- | --- |
| Host helper contracts | Two tests pass, covering completion, already-ready state, original error/prompt propagation, missing state, premature completion and bounded stalls |
| Static checks | Strict PoB all-target Clippy, final regression-target Clippy, workspace formatting and 46-document/496-link audit pass |
| Independent reference refresh | Both standalone generators succeed. All six fresh outputs match every non-provenance field of the original goldens, including 138 numeric measurements; input hashes match |
| Production regression | Self-contained public-runtime parent test passes; its intentionally ignored child helper executes. Caller items export required levels 49 without an authored level and 60 with a higher authored level. Both CLI calibration targets pass, covering all six Spark/Mace fixtures |
| Caller breadth | All five fresh reference evaluations pass: 110 measurements, 116 source-loading item reports, all 834 source instructions and full source projections remain exact. All five native full-build rejections remain explicit |
| Publication | Repair `45ed371` is integrated on main. Hosted validation remains pending; preceding combined-branch CI 34383255010 is separate |

Evidence: `runs/unique-readiness-phase-1.json`,
`runs/unique-readiness-witness-phase-1.json`,
`runs/unique-readiness-witness-lookup-1.json`,
`runs/unique-readiness-witness-drain-lookup-1.json`,
`runs/unique-readiness-witness-provenance.json`,
`runs/unique-readiness-production.export.xml`,
`runs/reference-readiness-helper-tests.log`,
`runs/reference-readiness-lib-clippy.log` and
`runs/reference-readiness-independent-final.json`,
`runs/reference-readiness-all-targets-clippy.log`,
`runs/reference-readiness-docs-audit.json`,
`runs/reference-readiness-calibration-tests.log`,
`runs/reference-readiness-cli-freeze.json` and
`runs/reference-readiness-corpus-final.json`,
`runs/reference-readiness-cli-validation-final.json` and
`runs/reference-readiness-oracle-final.md`/`.json`.
Independent callback-lifecycle review found no blocker. All local validation sessions
are terminal. The public regression observes exported requirements; internal natural-level
observations remain separately labeled probe evidence.

The repaired corpus changes its adapter fingerprint from
`c129b689c5de693b184fb40e130abea2e7f1078bbb4676a6fb9dade6a69bbe67` to
`ea91cc1dd287936ceee4e3c2c1a686164adfc179013f112639283894f81c126b`.
Diagnostics now record completion of both databases; startup timing, elapsed time and XML
child order can differ. Exported XML trees compare equal when disregarding child order;
no byte-identical export claim is made. The runner exits 1 only for the five unchanged
native whole-build rejections. Original corpus and frozen binary/data fingerprints match.

Resume: inspect the integrated main CI run and fix any observed hosted failure. Local
validation is complete; no broad local suite needs restarting without new changes or failures. The read-only next-slice proposal is
`runs/unique-native-seam-audit.md`: exact native lookup over injected, completed requirement
facts needs explicit source identity/readiness, canonical keys, fallback policy and
nil-preserving requirement semantics. The data-seam audit also requires constructor
insertion/missing-base evidence and detection of exact collisions or cross-entry fallback
dependencies before claiming traversal-order independence. Raw prototypes alone are insufficient; full native
prototype construction remains separate unfinished work. Do not infer support for unique
numerical effects from requirement lookup.

## General item formatting - locally validated checkpoint

The reusable [general item formatter](item-formatting.md) is implemented in the detached
worktree `C:/code/poe-optimizer/runs/item-formatting-worktree`. Data/extraction, the pure
engine, explicit parser resumption, native loading provider and caller CLI are integrated.
The main checkout was kept unchanged throughout local full-workspace regression session
51809 (now passed). All 26 protected baseline files remained unchanged there; the formatter
worktree deliberately changes only the package and one test-only Cargo.lock edge among them.

The schema-15 package has 25 sections and 12,747,598 bytes, SHA-256
`745e52fb8fcbb5c6292ad2f0b2abe17a7438dd3d021006697408c870d4b40329`.
It adds 15,090 exact-case keys, 12,040 capture records, 33 ordered partial format assignments
and source-derived defaults/catalyst policy. All 24 preceding section values and digests
remain exact. Existing actor precision and catalyst matching definitions are reused. Final
independent extractions C/D reproduce package and evidence bytes; their evidence digest is
`52059d23951244d8c370c5cf64ada9cbbb7d7af53d6c93bccffcb061ff26d220`.
The source pin, all 111 authenticated source files, old 87-rule formatting section, caller
exports and independent goldens remain fixed (`runs/formatter-data-preservation.json`).

The formatter returns exact text or an explicit bounded continuation requiring ordered
modifier-parser feedback. The loader retains calls made within formatting separately from
its subsequent modifier parse, with one shared sequence. Invalid or contradictory provider
feedback is rejected; unavailable dependencies, source errors and resource limits remain
distinct. The built-in provider supplies native formatting and injected catalyst scaling,
while general parsing and full assembly remain explicitly unavailable. The loader fingerprint
includes that provider and every engine formatter source file. The PoB oracle adds one
engine dev-dependency, with no package version changes or native runtime dependency.

Source review corrected repeated/Unicode suffix handling and distinguished Lua's indexable
nested string/array feedback from nonindexable values. Balanced advanced-copy enums remain
an explicit pending preprocessing boundary. Initial parsing still uses range 1, with authored
range selections retained separately for later assembly. No native build admission expands.

| Validation | Current evidence |
| --- | --- |
| Original formatter | All **13 independent tests pass**, strict scoped Clippy and formatting pass. Checks cover all catalog keys, 30,180 exact known-key strings, 8,094 raw formatValue/tostring cases, 3,000 applyValueScalar cases, 2,184 range/error observations and 10,944 catalyst cases. `runs/formatter-oracle-final-summary.json` records commands, counts and frozen hashes. |
| Parser and loading integration | Original execution covers **116 corpus items / 592 formatter calls / six precision parses**, plus 24 LF/CRLF provider cases and explicit nested/ordered feedback, suffix and enum cases. Live traces prove original formatValue and applyRange; applyValueScalar callbacks are not separately claimed as traced. These are formatting/preassembly comparisons, not a Rust BuildModList or full-build parity claim. |
| Native component contracts | **9 engine contracts and 218 import tests pass**; strict engine/import all-target Clippy passes. Source/resource errors, invalid provider traces, custom keys, selected ranges, nested enums and exact suffix ordering are covered. `runs/formatter-native-freeze.json`, `runs/formatter-import-all-targets.log`. |
| Caller integration | **19 selected inspection tests pass**, including a custom data key changing parser input without granting mechanic support. **29 corpus-runner tests pass**. Raw loading state remains bounded diagnostic metadata under existing report/runner schemas. `runs/formatter-cli-inspection-tests.log`, `runs/formatter-runner-contracts.log`. |
| Existing numerical and export behavior | Both complete actor/build comparison tests and all three package-extraction CLI tests pass, including two fresh complete 25-section exports. The extraction test was subsequently renamed to reflect the section count; assertions are unchanged. `runs/formatter-cli-extraction-build-parity.log`. |
| Workspace/portable checks | Strict workspace all-target Clippy, native-only Clippy, formatting, all five WASM library checks, **115 native-only CLI and 69 native tests** pass. The normal native-only dependency graph excludes PoB/Lua. Session 95232 completed with exit 0. `runs/formatter-native-deployment-summary.json`, `runs/formatter-workspace-clippy.log`, `runs/formatter-native-final-clippy.log`, `runs/formatter-wasm-final-checks.log`. |
| Fresh complete corpus | All five inspections and five reference calculations pass; **110 PoB measurements and all represented build/context/coverage/warnings are bitwise unchanged**. All 116 items still identify pending dependencies: 51 now reach modifier parsing after native formatting, with 24 rune, 18 crafted-affix, 15 base-buff, four Ward-header and four assembly stops unchanged. Native rejects the same three one-Skill and two one-SkillSet cases. Runner exit 1 represents these expected native rejections, not an inspection/reference failure. `runs/formatter-corpus-final/index.json`, `runs/formatter-corpus-validation.json`. |

The final read-only source/provider/data review found no additional blocker; raw typed API
limits and custom-provider provenance remain explicit. All **98 data tests and four
extraction tests pass**, with strict data/PoB Clippy. The data run corrected one stale v14
expectation to v15, preserving passed results and resuming the failed/unrun cases; source
and golden expectations were not relaxed. The final corpus uses rebuilt binaries after a
module-order rustfmt adjustment. All 139 captured production/data identities remain fixed
through that refresh and corpus run (`runs/formatter-final-binaries.json`,
`runs/formatter-validation-source-final.json`, `runs/formatter-freeze-format-adjustment.json`).

Final preservation and documentation checks pass: 24 nonexception protected worktree files
remain exact, all 139 captured implementation/data hashes are unchanged, and 45 documentation
files contain 477 resolving local links (`runs/formatter-final-preservation.json`,
`runs/formatter-docs-audit.json`).

Publication: code `48983fa7f57ec66d26ab9e6525a01fd3b97e55f5` is pushed to main.
[Exact-code CI run 34327347326](https://github.com/Azaril/poe-optimizer/actions/runs/34327347326)
passed on both platforms, as did prior repair run 34321961512 (confirmed 2026-09-09).
`runs/parser-prior-ci-34327347326.json` and `runs/parser-prior-ci-34321961512.json`
record completion; `runs/formatter-publication.json` retains the earlier observation.
There are no remaining local test/build sessions from this checkpoint. Preserve the frozen
worktree because the recorded binaries were built there. Main integration preserves all
24 nonexception protected files and the package bytes; remaining differences from captured
worktree files are only checkout newlines (`runs/formatter-main-integration-preservation.json`).

Resume by checking these exact hosted runs and any actionable failure annotations, then
continue the parser slice below. This publication-only document update can accompany the
next code checkpoint rather than dispatching a duplicate docs-only CI run now.

Next implementation: the general modifier parser's matching/dispatch seam, using the
read-only source audit in `runs/formatter-next-parser-audit.md` and `.json`. First capture
complete constructed dictionaries and their source/declaration evidence, then prove a
bounded byte-oriented matcher and the ordinary driver over injected records. Preserve
scan priority, retry replacement, sparse metadata merge and wrapper/tag transfers. A
selected unimplemented callback must defer explicitly; do not add a stat-name or corpus
allowlist. Keep nonfinite/function-valued parser results unresolved until the result model
can represent them honestly. This work uses the existing provider/formatter boundary;
full item assembly and numerical admission remain later consumers.

Full native actor/action/item calculation remains unfinished. The B3 general-model migration
remains a separate design discussion and is not implemented by this checkpoint.

## Ordered native item loading - validation checkpoint

The preceding goal turn was progress: mixed-store code `5f8b70e` and resume update
`316ad8c` were published. This phase started from a clean worktree and preserved the
source/build/golden baseline. Interrupted validation resumed on 2026-09-09. The preceding
exact-code runs 34274132680, 34271992799 and extraction-fix run 34269405262 now pass on
Windows and Linux (`runs/item-loading-prior-ci-final.json`). The user has authorized CI
log access for future failures. The full implementation/native-parity goal remains active.

The [ordered item-loading seam](item-source-and-loading.md) now has a complete injected
catalog and a portable Rust state machine. Schema 14 (`poe2-native-profiles-v14`) adds
1,756 item bases, including 251 hidden entries; nine modifier groups with 9,369 records;
30 raw unique groups with 443 prototypes; raw jewel radii and source-derived loading
policies. Opaque callback descriptors preserve source provenance without executing Lua.
Raw unique prototypes are not a constructed unique database or native mechanic support.

The selected bundle is **11,417,402 bytes**, SHA-256
`04ae73e51340a7ffacea213f4ac4bdf403bf71301d007949f4b55043c8120b07`.
All prior 23 section values and digests remain exact. The item catalog authenticates 87
source files; the full extractor authenticates 111. Two fresh review extractions reproduce
package and evidence bytes. Independent source comparison found and corrected an empty
noncorruptible-policy selector; reproducibility alone would not have detected that bug.
Six oversized source prototypes require a path-specific maximum of 64 KiB; ordinary string
limits and the 16 MiB package limit remain enforced.

Loading preserves every source occurrence and consumed instruction, including empty
construction, repeated text resets and retained fields, numeric/header syntax, variants,
parser-controlled line consumption, category/range order and final assembly. Explicit
provider results carry requirement replacements and modifier payload updates back to later
steps. Provider result sizes, finite numbers and list counts are checked before applying
those updates. Detailed traces belong to import/inspection, not the candidate hot loop.

The production provider still lacks general modifier parsing, range formatting and complete
item assembly. Advanced copied affixes, runes, unique construction, magnitude processing,
crafted reconciliation and other unresolved operations stop execution explicitly. The CLI's
schema-3 `inspect-build --with-definitions` / `--data` report preserves the established state
and marks the suffix unexecuted. Source-only inspection remains independent of data and PoB.
The schema-5 corpus runner checks occurrence ownership, consumed order, text hashes, source
and data identities, stop/error semantics and loader fingerprint. It never converts this
evidence into calculation capability. Custom provider provenance remains the host's duty.

| Validation | Evidence and result |
|---|---|
| Injected definitions | All **94 unique data tests** pass across 13 integration targets; the final stale schema assertion was corrected and rerun. Strict all-target data Clippy passes. Three authenticated extraction tests pass. `runs/item-loading-data-all-tests.log` retains the initial assertion failure; combine it with `runs/item-loading-data-resumed-test.log`, `runs/item-loading-data-clippy-resumed.log` and `runs/item-loading-source-final.log`. |
| Original item execution | **12 oracle tests pass**, strict scoped Clippy passes. The original runtime loads **116 items / 15 saved sets**, with 232 ParseRaw calls and 348 assembly calls. Of 486 authored range instructions, 402 write and 84 are ignored; no rune-list write occurs. Plain and observed source runs retain equal final item/list/assembled/set states. `runs/item-loading-oracle-frozen-tests.log`, `runs/item-loading-oracle-final-clippy.log`. |
| Native loading contracts | All **21 final loader contracts** and strict import Clippy pass, covering provider text/metadata bounds, wasm32-safe aggregate accounting and cross-item report limits. Earlier full import regression passed **203 tests** before the final resource guards. `runs/item-loading-final-bounds-checks.json`, `runs/item-loading-native-checks.json`. |
| Native/source transition parity | The same oracle suite compares represented scalar presence, requirements and complete modifier payloads across repeated text, parser retries, variants/version/groups, numeric/error cases and stopped formatting. Warmed execution proves surviving traces originate in original Item methods. Assembly replay uses freshly captured original results in test-only providers; it is **not a Rust BuildModList implementation**. |
| CLI and corpus contracts | **29 Python runner contracts pass**, including negative source/data/order/suffix cases, no-op instruction stops and aggregate diagnostic limits. All **21 selected-data CLI tests pass**, including two complete 24-section extractions. Final corpus evidence is recorded below. `runs/item-loading-runner-final-tests.log`, `runs/item-loading-cli-final.log`. |
| Fresh complete corpus | All **five inspections and five PoB evaluations pass**. All **110 measurements** and represented build/context/coverage/warnings match the previous checkpoint bit-for-bit. All **116 items** report explicit pending dependencies: 51 range formatting, 24 rune reconstruction, 18 crafted affixes, 15 base buffs, four unsupported Ward headers and four assembly. These are first stops, not complete dependency counts. Native still rejects three builds at one-Skill and two at one-SkillSet; the runner's exit 1 records those five expected rejections. Source/configuration/skill inventories and both binary identities remain unchanged during execution. `runs/item-loading-corpus-final/index.json`, `runs/item-loading-corpus-validation.json`. |
| Existing numerical behavior | Both established actor/build parity tests pass with the final data package. `runs/item-loading-actor-build-parity.log`. No native build admission is expanded. |
| Portable deployment | **69 native tests**, **114 native-only CLI tests**, strict native-only Clippy, all five WASM libraries and the normal-dependency check pass. The native-only dependency graph excludes PoB/Lua. Workspace all-target Clippy also passes. Broad suites precede the final resource-only guard edits; final loader/source tests, executable rebuild, native-only lint and five-library WASM refresh also pass. All 266 captured source/artifact hashes remain stable through that refresh. `runs/item-loading-deployment-summary.json`, `runs/item-loading-workspace-frozen-clippy.log`. |
| Preservation | **25 non-package protected files** remain exact; all old 23 package sections and digests are unchanged. Pinned PoB, tree, caller exports and six independent calibration goldens remain fixed. `runs/item-loading-final-preservation.json`, `runs/item-loading-root-final-package-validation.json`. All 44 documentation/notice files decode as UTF-8, 467 local links resolve, formatting and diff checks pass (`runs/item-loading-docs-audit.json`). |
| Publication | Code `43251c748ce735783bb0df6357f54f16d4134e35` is pushed to main. [Exact-code CI run 34320179629](https://github.com/Azaril/poe-optimizer/actions/runs/34320179629) failed at Lint on both platforms before tests. Local 1.98.1 reproduction found the collapsible-match diagnostic; the repair checkpoint below retains exact evidence. `runs/item-formatting-prior-ci.json`, `runs/item-formatting-stable-clippy-before.log`. Full native parity remains unfinished. |

Next resume point: check exact-code run 34320179629 and diagnose any failure from its logs, then
continue B2 dependency closure with reusable native modifier parsing/formatting and item
assembly components. The next bounded source port is exact ItemTools range/catalyst text
formatting: inject the complete case-sensitive scalability keys, per-capture scalability,
ordered format directives and precision policy. Preserve the separate initial range-1
formatting call and later selected-range assembly calls. Fallback precision discovery calls
the modifier parser; expose that dependency explicitly and stop when unavailable. Cover
negative ties, signed zero, forced decimals, numeric specialization, nested modifier
precision, missing/zero catalyst quality and unscalable tags against original source.
The existing 87-rule rangeless formatter is not complete general formatting. Measure the
expanded package before choosing limits. Keep the complete catalog independent of mechanic
admission and test new preparation stages before expanding complete-build coverage.
B4 whole-build holdout planning remains open. The proposed B3 actor/action/build/candidate
migration still requires discussion and is not implemented by this component.

## Mixed modifier stores - validation checkpoint

The preceding goal turn was progress: shared condition code `d09252b` and resume update
`ba8343c` were published. This turn started from a clean worktree and reverified all 26
protected inputs/data/goldens. The prior exact-code and extraction-fix hosted runs remain
live and testing; this checkpoint does not claim their completion.

Actual action layers use ModList above actor ModDB stores. Native numeric and condition
programs now preserve every layer's explicit kind, with complete kind-chain compatibility
checks. ModList SUM uses prefix-only source matching and errors on reached absent sources;
ModDB retains exact-or-prefix matching and skips absent sources. The FLAG source bypass
belongs to each producing ModDB layer and does not bypass a ModList parent or child.
Predicates retain the originally queried actor/store context across parent layers.

All constructors share an explicit **256-layer implementation bound**. SUM evaluates local
subtotals child-first, then combines them in parent grouping using fixed scratch space.
This preserves both error traversal and floating-point arithmetic without successful-query
allocations. Compiled condition kind chains make validation linear in layer count. Legacy
actor inputs remain ModDB and retain their compiled fast path. Stateful/global-limit tags,
opaque modifier values and complete actor/action preparation remain unimplemented; the
B3 general-model proposal is unchanged. No additional complete build is admitted.

| Gate | Current evidence |
| --- | --- |
| Mixed contracts | **7 numeric / 4 condition tests pass**, covering constructor bounds, source rules, exact kind-chain binding, child-first evaluation, actual parent references and allocation-free SUM. `runs/mixed-store-contract-final.log`, `runs/mixed-store-root-final-contracts.log`. |
| Original-source query parity | **9 tests / 8,762 paired observations pass**, plus **two separate direct error-order checks**. Historical numerical observations are preserved; the old compiled-source claim is superseded by the live-trace repair checkpoint above. Negative cells require the exact source failure and native MissingSource location. `runs/mixed-store-oracle-final-tests.log`. |
| Captured breadth | The new suite replays **15 represented condition closures and two isolated numeric rows** from the unchanged captured corpus. Four unresolved closures remain explicit. Frozen post-MAIN context is not complete native build preparation. Original source-only Tabulate/malformed-value controls are identified separately from native parity. |
| Existing query/actor regression | **65 modifier parity tests**, **13 condition-source tests**, **6 condition contracts** and **21 actor contracts** pass. The final condition/actor runs include the compiled kind-chain optimization and preserve varied candidate allocation checks. `runs/mixed-store-existing-parity.log`, `runs/mixed-store-root-final-contracts.log`. |
| Full-build regression | **2 actor build parity tests pass**, including fresh mapping/bossing Spark, composed Mace, condition passes, source removal and inherent flags. These remain the existing bounded profiles. `runs/mixed-store-actor-build-parity.log`. |
| Deployment | **69 native / 111 native-only CLI tests pass**. Strict native-only and workspace/all-target Clippy, formatting, five portable WASM libraries and the normal no-PoB/no-Lua dependency graph pass. Source hashes are unchanged across deployment checks. `runs/mixed-store-deployment-checks.json`, `runs/mixed-store-workspace-clippy.log`. |
| Preservation/publication | **26/26 protected files match** and the pinned PoB checkout remains clean. Four changed Markdown files decode as UTF-8 and 133 local links resolve. Code `5f8b70e220e2bd19ed79d0e3b6979a2347970ab2` is pushed to main. [Exact-code CI run 34274132680](https://github.com/Azaril/poe-optimizer/actions/runs/34274132680) is in progress. `runs/mixed-store-preservation.json`, `runs/mixed-store-docs-validation.json`, `runs/mixed-store-publication-ci.json`. |

The independent item-loading audit exercised original ItemsTab.Load, Item.ParseRaw,
ModParser and BuildModList for all **116 inventory occurrences** without replacing parser
results. It observed **232 ParseRaw calls** (116 empty constructor calls), **727 parser
calls**, **32 two-line attempts**, and **47 partial/unparsed remainders on 24 items**.
There are **43 rune-bearing items / 76 rune names**, plus advanced-copy and crafted cases.
Two fresh instrumented captures reproduced item-loading evidence byte-for-byte, and each
complete build snapshot matched the preceding uninstrumented reference bitwise. The
original runtime contains **1,756 item bases**. These observations guide the
[ordered loader and injected-definition seam](item-source-and-loading.md#ordered-loading-and-injected-definitions);
they do not constitute a native parser. Variants/groups/catalysts and repeated authored
strings still require independent source fixtures. Local source anchors and next tests are
in `runs/mixed-store-item-loading-audit-source.json`. Of the **486 ModRange instructions**,
**402 address existing line buckets and 84 exceed the loader's lists**. All requested values
are 0.5 and no final range-field changes were observed; this is not evidence that every
instruction was applied. The final audit is byte-reproducible from its unchanged captures.

Consolidated evidence: `runs/mixed-store-validation-summary.json`.

Generic MORE still uses a per-layer vector; this checkpoint does not establish an
allocation-free contract for every query or a whole-build throughput result. Ordinary
numeric values and represented predicates do not imply all source modifier kinds/tags.
Full native coverage, original/held-out full-build parity and realistic search remain open.

Next: inspect exact-code CI **34274132680** and the still-live preceding runs
**34271992799 / 34269405262**, diagnosing any actual failures from their bounded annotations.
Then implement the ordered item-loading boundary
against injected definitions and explicit parser outcomes. Continue B2 producer/dependency
closure and B4 whole-build holdouts. The pending
[general model proposal](general-build-input-proposal.md) must be discussed before its
actor/action/candidate migration; the item and query components can advance independently.

## Native condition producers - validation checkpoint

The preceding goal turn published `5b2ac70` and resume update `ec17180`. This turn began
with a clean worktree. The concrete hosted extraction timeout was diagnosed and its
functional-test fix published as `c7fb7fe`. The full native replacement goal remains active.

The [shared native condition component](native-condition-producers.md) implements ordered
ModDB FLAG/GetCondition producers over caller records and explicit actor/store references.
Resolver-aware numeric queries share raw scalar truthiness, parent/actor lookup, source
filtering, overrides, skill/weapon context and ordinary StatThreshold checks. Immutable
programs index producer names in source order; runtime bindings borrow candidate values.
A bounded local stack reports reached recursion and unsupported dependencies explicitly.
No Lua runtime or subprocess is involved in native queries.

The existing actor preparation now uses this resolver. Its compiled actor/candidate path
retains the existing allocation-free loop. The API migration is limited to a condition
resolver; no actor/action/candidate schema migration or new complete-build admission is
implied. The B3 general model proposal remains under discussion.

The corpus audit records **179 unique FLAG rows** and 19 query proposals. **15 represented
closures** are replayed using actual records and frozen post-MAIN context; **four remain
unresolved** (IgnoreCond, multiplier-threshold/provider context and weapon exceptions).
The checked-in fixture retains source/runtime/provider identities and all exclusions.
It does not depend on ignored run files at test time. ModList replays use only the common
unfiltered semantics; arbitrary ModList source-filter behavior is not yet represented.
Resolved stat snapshots are explicit test inputs, not native stat-production claims.

| Gate | Current evidence |
| --- | --- |
| Core contracts | **6 pass**, including raw value kinds, context compatibility, explicit unsupported/cycle errors, immutable sharing and zero-allocation binding/FLAG/GetCondition/producer-aware SUM. Indexed lookup retains order with 4,096 irrelevant producers. `runs/condition-query-contract.log`. |
| Original-source query parity | **13 pass / 1,020 paired query observations** at this historical checkpoint. The later [trace-evidence repair](#condition-trace-evidence-repair) qualifies the original compiled-source claim; numerical observations are retained. Includes 15 captured closures in both modes; four unresolved cases stay excluded. `runs/condition-source-corpus-test.log`. |
| Existing modifier and actor regressions | **65 modifier parity tests** and **21 actor contract tests** pass, including varied compiled candidate allocation checks. `runs/condition-query-engine-regression.log`, `runs/native-conditions-actor-tests.log`. |
| Complete-build differential checks | **8 pass** across actor, body-armour/movement and local-armour suites against original PoB. These are bounded existing profiles, not additional broader-build admissions. `runs/native-conditions-build-parity.log`. |
| Native deployment | **69 native tests / 111 native-only CLI tests** pass. Strict native-only and workspace/all-target Clippy pass. Five portable libraries compile for WASM; normal native-only dependencies contain no PoB/Lua packages. `runs/native-conditions-deployment-checks.json`, `runs/native-conditions-workspace-clippy.log`. |
| CI repair | The extraction functional test's three cases pass with the explicit 120-second test budget. Hosted run **34269405262** is still testing on both platforms. `runs/ci-extraction-functional-fix-evidence.json`. |
| Fresh corpus | All **five source inspections and five PoB evaluations pass**; **110 measurements** and build/context/coverage evidence repeat bitwise. The same five native rejections remain: one-Skill limits for builds 1/3/4, one-SkillSet limits for 2/5. Inputs, XML, configuration source and item/skill inventories are unchanged. `runs/native-conditions-corpus-validation.json`. |
| Preservation/publication | **26/26 protected file hashes match** and the pinned PoB submodule remains clean. Five changed Markdown files decode as UTF-8 and 175 local file links resolve. Code `d09252bc22a2f0a8788274e52b3275ae1885f5d6` is pushed to main; [exact-code run 34271992799](https://github.com/Azaril/poe-optimizer/actions/runs/34271992799) is in progress. `runs/native-conditions-preservation.json`, `runs/native-conditions-publication-ci.json`. |

Successful generic queries are not all allocation-free: ordinary MORE still uses a
per-layer vector. Whole-build preparation, dependency scheduling and realistic parallel
search costs need separate measurements on broadly admitted builds. Full native PoB
parity, complete input resolution and independent whole-build holdouts remain unfinished.

Consolidated local evidence: `runs/native-conditions-validation-summary.json`.

Next: inspect exact-code CI **34271992799** and the extraction-fix run **34269405262**,
resolving any concrete failure. Continue B2 producer/dependency closure
and B4 whole-build holdout planning. The new primitive does not authorize freezing live
conditions from PoB or silently ignoring unsupported producers. Continue independent
source loading/injected definitions; discuss the pending
[general build model proposal](general-build-input-proposal.md) before that migration.

## Ordered item-source projection - validation checkpoint

The preceding checkpoint `ee5f589` and resume update `ecaf0c8` are published. This turn
began with a clean worktree and retains the original inputs, pinned source, package and
goldens. The full native-parity goal remains active and incomplete. The B3 actor/action
architecture proposal remains pending; this checkpoint implements an independent input
component without starting that migration.

The portable item projection preserves every authored Item, saved equipment set, slot,
ModRange and Tree/legacy-Spec jewel occurrence with exact source ownership. It retains
raw text/comment/CDATA/processing-instruction fragments separately from the ordered
records consumed by the original XML reader. A later ParseRaw may replace earlier range
state, so the input cannot be flattened to one item string with ranges applied afterward.
Item loading, definition resolution, equipment selection and game effects remain later
stages; base/modifier/slot rules will come from injected data.

`inspect-build` report schema 2 exposes item source evidence independently of configuration
and skills. Item loading is `not_run`, equipment resolution is `not_resolved`, and passive
allocation is `not_checked`. The source-only lexical gate accepts the modeled mixed-text
cases; strict native/reference admission is unchanged. Corpus manifest schema 4 preserves
schema-1 reports as lacking item evidence, rather than treating absent fields as empty
inventories. It validates ordered records and source ownership against the caller XML.

The B2 static capability audit retains 3,469 per-build / 2,162 cross-build unique modifier
records and 53,824 owner/layer uses. It distinguishes query primitives from producers and
complete native pipelines. The 1,559 representable primitive numeric records are not a
support percentage; complete conditions, targeted stats, named intermediates, provider
transfers and dependency closure remain open. See the
[capability findings](breadth-mechanism-inventory.md#observed-native-dependency-capabilities).

| Gate | Current evidence |
| --- | --- |
| Portable import | **187 tests pass**, including 11 initial item-source cases. The final focused item suite passes **12 tests**, including an additional namespace-parser rejection regression; no new full-import suite run is claimed. `runs/item-source-import-focused-final.log`. Strict import Clippy passes. `runs/item-source-import-tests.log`. |
| Original-source comparison | **9 tests pass** against the original XML reader and ItemsTab/PassiveSpec load methods, across all five builds and synthetic ordering cases. ParseRaw/GUI boundaries are explicitly instrumented; no numerical item parity is implied. Interpreted/JIT-enabled modes are not claimed to be warmed traces. `runs/item-source-oracle-tests.log`. |
| CLI | **111 native-only CLI tests pass**, including all existing bounded native behavior; **15 feature-enabled inspection tests pass**. `runs/item-source-native-only-tests.log`, `runs/item-source-cli-tests.log`. |
| Runner/corpus | Fresh schema-4 inspection passes for all five original builds: **116 items / 15 item sets / 486 ModRange records / 16 passive specs / 21 jewel assignments**, plus all **15 skill sets / 200 groups / 541 gems**. All **110** reference measurements and build/context/coverage evidence repeat bitwise. The existing five native rejections remain; mixed-run exit 1 is expected. `runs/item-source-corpus-validation.json`. **23 Python contract tests pass**, covering omitted/altered text, forged source slices, malformed attributes/roles, namespace/source bounds and the item depth boundary. `runs/item-source-runner-tests.log`. |
| Hosted diagnosis | Prior exact-code run 34262898483 failed on both platforms. Four immediate targets after the last visible success pass locally: equipment parity 1, evaluation options 7, evaluator 4, extraction 3. Actual hosted assertion remains unknown. `runs/item-source-ci-targeted-tests.log`, `runs/item-source-ci-extraction-tests.log`. |
| CI diagnostics | Published `88654d1` bounds individual annotations below the public 4096-character limit, newest chunk first. Four controlled cases pass, maximum 3907 characters, correct streaming/exits and simulated truncation. `runs/ci-annotation-chunks-validation.json`. |
| Integration gates | Formatting, strict workspace/all-target and native-only Clippy, five portable WASM libraries and the no-PoB/no-Lua normal dependency graph pass. The final four inspection targets repeat **15/15** passes. `runs/item-source-final-*`. |
| Preservation/publication | **26/26 protected raw file hashes match**, and the pinned PoB checkout remains clean. Seven changed Markdown files decode as UTF-8; 186 local links and 19 heading targets resolve. `runs/item-source-final-preservation.json`, `runs/item-source-final-docs-validation.json`. Code `5b2ac700c01412b1778194b4b94ed291a6282c36` is pushed to `origin/main`. [Exact-code CI run 34268455262](https://github.com/Azaril/poe-optimizer/actions/runs/34268455262) is in progress; no hosted pass is claimed. The final corpus uses `runs/item-source-corpus-final`; all input and executable identities are retained in its manifest. |

Consolidated local evidence: `runs/item-source-validation-summary.json`.

Next resume point: inspect exact-code run **34268455262** and diagnostic-only run
**34266873370**, obtaining any concrete hosted assertion from the bounded annotations
before repairing its cause. Finish B2 dependency/producer closure and B4 whole-build holdout planning.
Source load resolution and injected item definitions can proceed independently of the
pending [general model proposal](general-build-input-proposal.md). Broad native build
support, full held-out parity and realistic parallel search remain unfinished.

## Breadth mechanism inventory and corpus integration - validation checkpoint

The preceding goal turn made concrete progress: code `19666a0` and resume update `7318300`
are published. This continuation began with a clean main worktree. The full native-parity
goal remains active and incomplete. The existing B3 architecture question remains pending;
this checkpoint implements independent corpus tooling and evidence, without changing the
actor/action/candidate architecture.

The [mechanism inventory](breadth-mechanism-inventory.md) now records all **116 authored
items, 15 item sets, 486 modifier-range records and 16 passive specs**. Three independent
audits bind item/source parsing, full-tree lookups and fresh MAIN actors/dependencies to
the original inputs and pinned data. All 1,369 authored node occurrences are accounted for:
549 supported native node views, 788 excluded views and 32 implicit roots. These are
node lookups, not complete-build admission or legality.

The six isolated ordinary notables in build 1 are linked by the live reference to the
**From Nothing jewel at socket 7960**. They are separate from ascendancy roots/budgets and
from the 14 source-missing targets; none of the five builds allocates a missing target.
The source graph includes class-to-ascendancy connectors, so raw component counts do not
define point categories. See the [topology investigation](tree-topology-investigation.md).

Fresh MAIN captures retain **94 prepared player actions, 25 minion representations, 68
minion actions, 14 item/tree grant links, 310 applied-support links, 47 constructed buffs
and one explicit trigger binding**. Conditional records are not automatically active
contributions. Five unique source-unknown passive IDs have empty actual parsed/final
modifier lists; that narrow source limitation remains distinct from missing Rust mechanics.

The reusable corpus runner now has `--inspect-build` and `--with-definitions`, using the
caller-selected import CLI and optional data snapshot. Manifest schema 3 retains source,
configuration, skill occurrence and identity counts alongside independent backend outcomes.
It checks input/data identities, source ownership, complete occurrence coverage and explicit
non-evaluation labels. Inspection failures stay separate; changed XML stops later work.

| Gate | Current result / evidence |
| --- | --- |
| Runner contract | **15 Python tests pass**, including unchanged-source/data bindings, independent failures and duplicate nested selectors. `runs/breadth-runner-source-tests-final.log`. |
| Real corpus | All five `inspect-build` reports succeed: **15 skill sets / 200 groups / 541 gems**, **537 exact external IDs + 1 explicit effect + 3 unprocessed names**. All **110** prior reference measurements and build/context evidence repeat exactly. Existing five native rejections remain; mixed-run exit 1 is expected. `runs/breadth-mechanisms-corpus-validation.json`. |
| Item/passive audits | All saved sets/specs retained; item audit reproduces byte-for-byte. Source and provider identities remain explicit in `runs/breadth-items-inventory.json` and `runs/breadth-passives-inventory.json`. |
| Independent MAIN audit | Five fresh original-source snapshots match previous build/player/minion/context/coverage/warnings, including float bits. Final captures `runs/breadth-dependencies-reference-4/index.json`; analysis and preservation `runs/breadth-dependencies-analysis-3/`. |
| Review | Independent script review found no blocker. Factual review corrected interleaved item-processing order and clarified that support/buff totals cover all prepared actions in active MAIN environments. |
| Additional local CI diagnosis | Complete PoB unit target: **60 pass**, three ignored child helpers; existing source data oracles: **19 pass**. Extraction CLI: **3 pass** even with this test process restricted to two logical CPUs; original affinity restored. No failure reproduced and no speculative Rust/timeout change made. `runs/breadth-ci-pob-library.log`, `runs/breadth-ci-pob-data-oracles.log`, `runs/breadth-ci-extraction-two-cpus.json`. |
| Prior hosted CI | Exact-code run **34257699126** failed on Linux; Windows was cancelled. Public metadata exposes only exit 101. The actual assertion remains unknown; a request for that log is pending. `runs/breadth-mechanisms-prior-ci-final.json`. |
| Static preservation | Six changed Markdown files pass UTF-8 checks; 190 local links and 17 heading targets resolve. Caller inputs, numerical goldens, data and pinned source are preserved; final CI workflow changes are checked separately. `runs/breadth-mechanisms-static-validation.json`. |
| CI diagnostic tooling | Both Cargo test commands retain streaming output and exact exit codes while publishing a bounded failure tail. Three subprocess checks pass for exits 0/7/7, stdout/stderr streaming, 120-line / 16,000-character bounds and annotation escaping. Matrix fail-fast is disabled so each platform retains its outcome. `runs/ci-annotation-wrapper-validation.json`. |
| Publication | Code `ee5f589ff27a3ecaa6d37aa7007bb10b45f5983d` is pushed to `origin/main`. [Exact-code CI run 34262898483](https://github.com/Azaril/poe-optimizer/actions/runs/34262898483) is in progress on both platforms. No hosted pass or repair of the preceding failure is claimed. `runs/breadth-mechanisms-publication-ci.json`. |

The full checkpoint evidence is `runs/breadth-mechanisms-validation-summary.json`.
CI diagnostic work adds bounded public failure annotations and independent platform outcomes;
it does not repair or reinterpret the earlier unknown failure. Stored-credential access was
rejected by automatic approval review; it was not retried. Normal browser initialization
also failed, and public job metadata does not include the assertion. Use user-provided logs
or the next run's annotations for a concrete repair.

Next resume point: inspect exact-code run **34262898483**, using its public failure
annotations for any concrete diagnosis. Then finish B2's native capability matrix for
the retained modifier/dependency records and plan B4 whole-build holdouts. Item source/load
instructions and injected parsing data can proceed independently of the pending
[shared actor/action model discussion](general-build-input-proposal.md). Do not begin its
architecture migration until that direction is settled. General build support, broad
held-out numerical parity and realistic parallel search benchmarks remain unfinished.

## Skill source projection and identity catalogs — validation checkpoint

This checkpoint continues from published root-container code `b109b94` and resume update
`2c19854`. Their exact-code CI remained in progress at the last read
(`runs/skill-source-prior-ci-final.json`). The full implementation/parity goal is active
and incomplete.

The B3 actor/action/candidate migration proposal has been presented for user discussion.
The delivered independent work preserves authored skill inputs and constructs an injected
identity catalog; it does not implement that broader architecture or widen numerical
admission. Delivered scope:

- Bounded, source-preserving Skills/SkillSet/Skill/Gem and nested selection projection,
  retaining ordered duplicates, unknown fragments, all saved sets and exact attribute bytes.
  Source-consumer roles are separate from syntactic names and effective/default state.
- Complete authenticated gem/skill identity construction from the pinned original source,
  with ordered declarations separate from constructed winners, generated additional effects,
  stat-set references and ambiguous lookup provenance. The schema-13 identity-only
  section preserves all 22 existing package sections exactly.
- Independent original SkillsTab Load/Save/set and Data.lua identity oracles. Pre-processing
  observations are labelled separately from ProcessSocketGroup/effective interpretation.
- Caller-driven `inspect-build --with-definitions` / `--data` integration over portable
  snapshots, without native compilation, PoB runtime, fixture fallback or effect admission.

Gem external game IDs, variants, internal keys and granted-effect IDs are distinct. An
explicit unknown gemId must not fall through to skillId; source `pairs` fallback cannot
become an invented portable sorted choice. Primary/additional global-effect flags are not
weapon-set numbers. Legacy selector attributes may be reset before nested maps load.
The source projection retains these facts without reproducing lossy overwrite/migration
as if it were authored input. The corrected identity artifact is 6,034,967 bytes, SHA-256
`91d72da5882d40c822044763e19e9894e26027bb8d97766b4d65b27e8e586acb`.
Its source spans reconstruct from advertised inclusive lines; an independent oracle caught
and prompted repair of the initial trailing-blank-line mismatch. All 22 old section raw
JSON values and digests, and the separate tree bytes, remain unchanged.

| Check | Final local evidence |
| --- | --- |
| Source projection and adapter | 13 source tests and 9 independently authored lookup tests pass. All saved sets, unknown/duplicate/legacy records, exact source values, precedence, ambiguity and report expansion limits are covered. |
| Portable data/import/native | **330 tests pass**, including 85 data tests, 176 import tests and 69 native tests. Unchanged independent Spark/Mace goldens and allocation-free prepared calculations still pass. Log: `runs/skill-source-portable-tests.log`. |
| Original-source comparisons | **9 tests pass**, cold/warm, covering all declarations/constructed identities and exact corpus LoadSkill/ProcessSocketGroup observations. Five focused extractor tests cover real construction, source spans, declaration scanner restrictions and metadata type coercion rejection. |
| CLI and extraction | **14 feature-enabled CLI tests pass**, including two fresh-process extraction runs and native loading of their output. The three extraction CLI tests pass again after final extractor hardening. **107 native-only CLI tests pass**. |
| Catalog reproduction | Two final fresh extractions reproduce all five artifacts exactly. Extractor SHA-256 `1407957e20ce437b64ef203a99a1f3fda63c582265db9a4e17d8e5f49605fee8`; 44 authenticated extraction files, including 17 identity-construction files. |
| Breadth | All five source/identity inspections pass: **537 exact external matches, one explicit effect and three name-only records left unresolved before processing**. All 110 fresh reference measurement records, build/context evidence and original XMLs exactly repeat the prior corpus. Native still rejects one-Skill/one-SkillSet layouts. |
| Integration gates | Workspace and native-only strict Clippy, formatting, five portable WASM libraries and the no-PoB/no-Lua native normal dependency graph pass. UTF-8 and modified-document links pass. This is **358 affected workspace-target tests**, plus the separate 107 native-only tests; no new full-workspace test run is claimed. |
| Preservation | All 22 old section raw JSON values/digests, 47 checked tracked files, six independent reference JSON goldens, both user input files and tree/source inventories are preserved. The pinned submodule remains clean. |
| Publication | Code `19666a0ad754ac5514553a71138a568729022d2e` is pushed to `origin/main`. [Exact-code CI 34257699126](https://github.com/Azaril/poe-optimizer/actions/runs/34257699126) later failed on Linux; Windows was cancelled. Diagnosis is recorded in the current checkpoint. |

Consolidated evidence: `runs/skill-source-validation-summary.json`,
`runs/skill-catalog-preservation.json`, `runs/skill-source-identity-corpus/index.json`,
`runs/skill-source-breadth-reviewed/index.json` and `runs/skill-source-breadth-comparison.json`.
Final logs use `runs/skill-source-*` and `runs/skill-catalog-*`; publication evidence is
`runs/skill-source-publication-ci.json`. The earlier
`runs/skill-source-breadth-final` exploratory run used the preceding native executable;
`breadth-reviewed` is the final rebuilt-binary evidence above. Repeated checks are not
added to the affected-test count.

Next resume point: review the already-presented B3 actor/action/candidate proposal before
migrating closed native profiles. The effective resolver must own saved-set overwrite and
fallback rules, ProcessSocketGroup name/level/requirement processing, independent MAIN/CALCS
stat/minion selectors, grants and actor/effect ownership. While that discussion is pending,
B2 modifier/passive/condition/dependency inventory and B4 held-out-family planning remain
independent useful work. Do not remove saved sets/supporting skills or route unsupported
builds through hidden PoB fallback. This slice does not claim any new numerical capability.

## Source build containers — validation checkpoint

The caller-driven `inspect-build INPUT [--output NEW_FILE]` command and portable immutable
root projection preserve arbitrary build containers independently of native skill coverage.
Unknown/duplicate sections remain diagnostic source; malformed Calcs scalar values retain
local errors rather than erasing other containers. The new shared MAIN gate replaces three
root allowlists in native evaluation, general controlled templates and legacy Mace templates.
Accepted bytes remain untouched through materialization/export and exact finalist checks.

Original consumer evidence distinguishes MAIN from CALCS, display layout from effects,
Import export flags from incoming Party payloads, and authored settings from Load migrations.
Only three strictly typed Calcs display inputs and known layout rows are admitted; all
Party payloads, exportParty=true, unknown fields, legacy Calcs keys and duplicate singleton
sections remain excluded. No new game effect or build-specific default was introduced.
The source contract and limits are in [source build containers](build-source-containers.md).

| Check | Current evidence |
| --- | --- |
| Source semantics | 10 independent original-source oracle tests pass; all five original auxiliary projections match original XML values. MAIN/CALCS, 26 legacy mappings, actual Party numeric/flag parsing and Load/Save ordering are covered. Aura/curse numerical effects are not claimed. |
| Generic projection | 12 independent tests pass: exact corpus hashes/order, arbitrary source, unknown/duplicate/namespaced shapes, scalar diagnostics and resource bounds. |
| Full-build differential | 18 metadata variants across Spark/Mace plus two baselines complete 20 fresh PoB calculations; all 13 requested metrics match native and unchanged baselines, with exact native XML exports. This is metadata parity, not gameplay parity for the five broad originals. |
| Caller-only inspection | Three new CLI tests pass, including all five XML/share-code pairs, execution outside the repository, missing-input rejection and no overwrite. |
| Broad originals | Five exact imports/configuration projections and five fresh PoB evaluations succeed. Native now reports one-Skill restrictions on lines 1/3/4 and one-SkillSet restrictions on lines 2/5. The mixed runner intentionally exits 1; no original build is newly admitted. |
| Final affected validation | 223 import/native tests, 23 original-source regression tests and eight relevant CLI tests pass (254 workspace-target tests). The full native-only CLI suite passes 103 tests. Workspace/native-only strict Clippy, formatting and five portable WASM libraries pass; native-only normal dependencies contain no Lua/PoB. This is affected-target validation, not a new complete workspace test run. |
| Publication | Code `b109b941dd7d6f7f928b69e27d7e651ffd507415` is on main. Exact-code Windows/Linux [CI run 34253146642](https://github.com/Azaril/poe-optimizer/actions/runs/34253146642) is in progress. `runs/root-container-published-ci.json` records the observed state; no hosted pass is claimed. |

Local evidence: `runs/root-container-semantics.json`, `runs/root-container-oracle-tests.log`,
`runs/root-container-projection-tests.log`, `runs/root-container-cli-tests.log` and
`runs/root-container-breadth-final/index.json`. The final corpus uses separate native-only and
optional-reference binaries. `runs/root-container-validation-summary.json` reconciles
all 110 exactly repeated reference measurements and source/data preservation. The manifest records exact commands,
input/data/binary hashes and independent outcomes. Schema-12 data, pinned tree/source,
user imports and six independent numeric goldens remain unchanged.
Integration updated two old empty-Party rejection cases to effectful Party payloads;
new positive tests cover empty Party. An unrelated existing Unicode test-text rewrite
was restored to HEAD bytes; 30 Mace import and eight native-contract tests were repeated
after restoration. The final affected suite passes and no unresolved failure remains.

The B2 skill/source inventory now covers all **15 saved skill sets**, **200 groups** and
**541 gem occurrences**: 537 exact external game-ID/variant matches, one explicit known
EnemyExplode effect and three retained name-only unresolved entries. It records 42 authored
source groups, 20 slot-assigned groups and 14 referenced minion definitions. A duplicate
literal SkeletalSniper catalog key and ten additional/minion action IDs without static
skill definitions are explicit source-construction caveats, not guessed replacements.
Identity recognition is separate from numeric support. Item-modifier, passive/condition and
complete dependency/legality inventory remain open under B2. Evidence is
`runs/root-container-skill-inventory.json`, SHA-256
`0d3f143279587c9e363b233c07e20c053cef550dd6eaa13e2cc2f3d5cab850d8`.

Next: review [the concrete B3 model proposal](general-build-input-proposal.md). User direction
was requested before this broader interface migration; the proposal is not yet accepted or
implemented. Independent source projection/identity inventory and remaining B2 audits can
continue while that design discussion is pending.
The root gate must not be bypassed by dropping settings, relabeling minions as player
skills, treating a saved-set index as an effect index, or silently choosing a simpler build.

## Injected configuration definitions — validation checkpoint

The new `configuration` package section contains the complete ordered source metadata,
independent of current skill profiles and corpus keys. An immutable `ConfigDefinitionCatalog`
indexes occurrences and keys once per snapshot, retaining duplicates. Import lookup consumes
that snapshot without `CompiledGameData`, native calculation or a PoB runtime. The optional
CLI `--with-definitions` / `--data` path exposes source/data-bound evidence; default inspection
remains data-free. Custom renamed keys, mixed typed options and changed defaults are tested.
The legacy four-fixture PoB materializer and frozen expected configuration were moved into
integration-test support; production input APIs no longer export that fixture-only registry.

The catalog retains 663 source table positions, 564 definitions for 563 keys, 142 typed
options, 17 generated quests and 540 inert callback descriptors. `initial_state()` reproduces
ordered `CreateConfigSet` assignments, including removal by nil and `defaultIndex` precedence.
It does not apply UI fallback, load migrations, dynamic placeholders or modifier callbacks.
A stored scalar's type can differ from expected authored-input metadata, as in Lua. Unknown
keys, unlisted options and scalar-kind mismatches are diagnostics, not silent data deletion.

Package schema **12**, semantics **`poe2-native-profiles-v12`**, has 22 sections, **4,427,011 bytes**,
SHA-256 **`ee37ded45933ec240d087dea7f270c0be8a30bf9a4d29cce46420a72df9b72bd`**.
All previous 21 payloads and section hashes, plus both tree artifacts, are unchanged. The
extractor authenticates 37 direct files, including original boss metadata and its tooltip
producer. Function locations use original source line information; callback code is not
serialized or executed by the native catalog. The private offline extraction envelope is
bounded to 17 MiB; the public package keeps its independent 16 MiB limit. Lookup additionally
limits total definition matches and option comparisons to 65,536 each before publication.

| Gate | Current result / evidence |
| --- | --- |
| Portable data | 74 data tests pass, including 17 catalog tests; `runs/configuration-data-all-tests.log` |
| Original-source catalog/default parity | 4 tests pass, cold and 100-repeat warm, all metadata/options/defaults/callback spans compared independently; `runs/configuration-catalog-parity.log` |
| Generic import lookup | 6 tests pass, including every scalar in all five originals, custom injection and expansion limits; `runs/catalog-definition-import.log` |
| CLI inspection / fresh extraction | 12 final CLI tests pass (5 new definition, 4 source, 3 extraction); both fresh extractions exactly reproduce the final package/evidence; `runs/catalog-final-cli-tests.log` |
| Native-only CLI | 100 tests pass and strict lint passes on the final package; `runs/config-catalog-native-cli-all-tests.log` |
| Combined workspace target coverage | 740 tests pass: 587 final library tests plus 148 existing CLI regressions and 5 new CLI tests. Final changed CLI targets rerun separately; see validation note below and `runs/catalog-validation-summary.json` |
| Test-only fixture registry | 5 tests pass; unchanged test bodies/assertions and no production module export; `runs/config-test-only-candidates.log` |
| Strict lint / WASM / isolation / docs | Final workspace and native-only Clippy, formatting, five portable WASM libraries, no-Lua dependency checks and 35-document/396-link audit pass |
| Breadth | All five sources and 228 scalar lookups preserved; five fresh PoB successes with 110 unchanged measurements, five explicit native root-layout rejections; `runs/catalog-breadth-summary.json` |
| Class/ascendancy roots | Existing native passive regression passes 100 fresh PoB evaluations; generated Spark/Mace identity cases, not held-out breadth |
| Corpus runner | 9 Python tests pass; `runs/catalog-corpus-tool-tests.log` |
| Numerical/tree preservation | All 21 original payloads/digests and tree bytes unchanged; source pin/user inputs/46 recorded files preserved; `runs/catalog-section-preservation.json`, `runs/catalog-source-preservation.json` |
| Publication / hosted CI | Code `87bf0056209653d9b46d922477e9cfe920aeb6b4` is on main; Windows/Linux run `34249708752` is in progress. Preceding source checkpoint run also remains in progress |

Validation accounting: the initial full-workspace run began before the metadata-only
function-span correction and test-only API cleanup. It completed all 148 existing CLI
regressions, then failed the new strict function-end assertion against its earlier compiled
catalog artifact. That draft retained the adjacent comment after `CreateConfigSet`; the
independent oracle had already identified and corrected this metadata span. The final
artifact records exactly lines 1317–1334. A separate final non-CLI all-target command passes
587 tests, including all four strict catalog oracles, and the final
12-test CLI rerun includes all five new definition cases plus source inspection and fresh
extraction. Thus the 740 total is combined target coverage, not a claim that one pristine
final full-workspace command produced it. Nine ignored harness entries are dedicated child
helpers invoked by their parent tests. The final package, original numeric values, goldens,
source pin and caller imports are all checked. A passing source-definition comparison is
neither full-build numerical parity nor support for a newly observed mechanic.

**Resume next:** continue B2 with a classified root-container projection
for Import, Party, Calcs and TreeView. All five builds currently reach this root-layout guard;
classify workflow/UI records separately from calculation-bearing party and selection data,
without ignoring unknown effects. Extend the complete mechanism inventory beyond configuration.
Then propose the shared actor/action/build model under B3 before significant architecture
changes. Effective configuration resolution and migration of native quest/encounter bindings
are subsequent work with independent source dispatch and full-build parity gates. Keep the
separate ordinary/ascendancy roots and budgets; the 14 missing targets remain source findings.

## Configuration source projection — validation checkpoint

The shared source/configuration reader, native/search integrations and caller-driven
`inspect-configuration` CLI are implemented. Source fields bind to the original XML with
exact raw ranges, one-pass decoded scalars, set/record order and requested/resolved selection
provenance. The same code serves arbitrary caller inputs; fixture names and quest names do
not select production behavior. Legacy actor text parsing reuses the shared primitive.

Source projection is separate from effective ConfigTab state and numerical admission. Unknown
configuration remains visible without granting native capabilities. Only projected
`Input.string` fields get the native literal-whitespace exemption; unknown fragments,
Placeholder attributes, keys and titles remain strict. Three scalar consumer paths now use
the shared reader. Native/backend evidence hashes include the new source modules.

The complete source audit resolves all 59 corpus keys, preserving 58 Inputs, 170 Placeholders
and one block. All five ConfigSets are active, with no inactive sets in these originals.
Source definitions have 564 variable rows for 563 distinct keys; duplicate callbacks and
separate default mechanisms are documented in the [catalog proposal](configuration-data-proposal.md).
A portable injected catalog and capability-aware resolver are still pending.
The follow-up root-container inventory identifies Import/Party/Calcs/TreeView as the next
shared guard. Calcs and Party can carry real calculation semantics; blanket UI-only ignoring
is inappropriate. See [container and selection barriers](breadth-validation.md#next-container-and-selection-barriers).
All five retain later multi-group/set and unsupported-mechanic barriers.
The [unresolved-label audit](breadth-validation.md#unresolved-labels-and-actor-ownership)
traces all three line-1 labels to name-only rows: Djinn actions require their summoning
owner, while Powered Zealot has two ambiguous monster identities. Existing reference
coverage preserves this distinction; no source substitutions or new native capability
were introduced.

Local validation is complete:

| Area | Evidence |
| --- | --- |
| Shared import | 118 import library tests and 9 new projection tests pass; source bounds, exact ranges, set/record order and strict native exemption scope are exercised. |
| Native integration | All 66 native tests, all 148 reference-enabled CLI tests and all 95 native-only CLI tests pass. Production inspection requires caller input and runs from a directory without a reference checkout. |
| Independent source oracle | 8 configuration tests and 5 relocated XML compatibility tests pass against original pinned Lua. ConfigTab tests use an empty default-variable list and stub UI callbacks; they do not claim full default or mechanic resolution. |
| Corpus | All five unchanged inputs import/project successfully and all five fresh PoB runs complete. Native reaches the restricted root-layout guard on all five. Exact source counts: 58 Inputs, 170 Placeholders, one block; no modified inputs/executables. `runs/config-breadth-validated-20260908/index.json` and `runs/config-breadth-summary.json` retain hashes/outcomes. |
| Runner integrity | Nine Python tests pass, including malformed source reports and inspection-time XML mutation/deletion. Failed inspection remains independent; changed decoded XML cannot reach a backend as the original source. |
| Tooling and portability | Workspace/native-only strict Clippy, formatting, all five portable libraries on `wasm32-unknown-unknown` and native-only Lua dependency exclusion pass. All 35 documents / 390 local links and Git whitespace checks pass. |
| Publication | Code `0da5c5c288442f1ac11cc5ffea0c948813dc6d9e` is pushed to main. Both exact-code [Windows/Linux CI jobs in run 34243706906](https://github.com/Azaril/poe-optimizer/actions/runs/34243706906) are in progress (`runs/config-main-check-runs.json`). The following publication/resume update changes documentation only. |

Logs use `runs/config-*`, plus `runs/configuration-source-parity.log` and
`runs/configuration-existing-xml-parity.log`. The preceding action-timing checkpoint remains
the full workspace numerical baseline; this phase does not claim a new full-workspace run.
The completed source/import/native targets plus all reference-enabled CLI targets pass 354
tests; the separate native-only CLI run passes 95. `runs/config-validation-summary.json`
reconciles the complete logs. No failures remain. Fresh corpus reference measurements also
repeat all 110 prior measurement records exactly, including availability/status.
No numerical data package, source pin, independent golden or original corpus byte changed.
The configuration audit artifact is `runs/config-definition-inventory.json`, SHA-256
`35a4bfc22905d5b8896ab8f3b0d007b8c2d6ad40e75169ba05069effb262fba4`.

Next: check exact hosted CI for `0da5c5c` and resolve any failure, then implement ordered
injected configuration definitions with source parity gates. Continue
B2/B3 inventories and the general build-model review; broad native mechanics and full parity
remain unfinished.

## Action speed, timing and breadth intake — validation checkpoint

The shared schema-11 pipeline, root CLI problem 11/report 12, and corpus intake are implemented.
All workspace targets, native-only CLI checks, isolated release measurements and fresh finalist parity pass.
The [action-timing guide](action-timing.md) defines exact semantics and producer limits.
The five original corpus inputs remain unmodified; they are not yet native-supported builds.

| Area | Evidence currently available |
| --- | --- |
| Data/source | Package schema 11, 21 sections, 35 direct source files, 359 actor rules, 87 formatting keys and 402 armour bases. All 1,282 prior passive views and 3,476 exclusions remain unchanged. Source checks include 6,000 action-speed cases, 5,184 complete timing cases, 4,020 grammar inputs and 696 formatting observations. |
| Engine | 116 engine tests pass, including original cold/warm MAX, positive-row sums and complete timing branches. A new loop prepares 8,400 changing actors and runs 16,800 skill calculations with zero allocations; the prior four-slot allocation loop now also varies action sources. |
| Import/native | 118 import and 66 native tests pass in an unfiltered final all-target run; Clippy passes. Native metric 14 is player action speed. MAX absence, source records, exact realization and explicit nonfinite derived timing remain distinct. Evidence: `runs/action-adapter-final.log`. |
| Complete builds/CLI | 20 fresh native/PoB pairs plus four fresh export/reimport pairs pass, including hand-specific CastRate/Speed/Time, shared movement, source integer grammar, zero speed, floors and the tick cap. All 18 graph CLI tests pass across typed/document and one/four workers. The benchmark metric count and one older local-weapon test's media-version selector were updated; all numerical assertions remain unchanged. Full workspace coverage is reconciled below. |
| Breadth | Five exact imported builds and immutable share strings are indexed, independently audited and exercised by the reusable corpus runner. All five reference runs complete; native admission rejects the multiline quest attribute. Six runner tests pass. Ascendancy ownership/root connectivity and separate core point budgets are already implemented; the 14 source-missing targets are different diagnostics. |
| Integrated checks | **689 passed**, nine source worker helpers exercised by parent tests, across **96** workspace target batches / all **81** metadata-declared integration targets. The native-only CLI passes **91** tests across 39 batches. Strict workspace/native-only Clippy, formatting, five-library WASM compilation and native-only dependency isolation pass. Python corpus tooling passes six tests. |

The first combined run stopped at an obsolete version-8 evidence selector in the older
local-weapon parity test. Correcting that selector to version 9 preserves every numerical
comparison; the full target then passes. Already passing unchanged CLI targets, the resumed
CLI targets and a complete independent library run are reconciled against Cargo metadata in
`runs/action-speed-test-coverage.json`. This is complete target coverage without repeating
unchanged passing tests. All remaining commands exited successfully.

The action phase exposed a pre-existing JSON transport defect: default serde_json parsing can
move finite values by one floating-point step, including ordinary movement evidence and
large/small custom values. Typed evaluation succeeded while document realization rejected
its own numerical evidence. The workspace now enables `float_roundtrip` consistently, with
bit-preserving core/data/native regressions. Numerical comparisons were not relaxed. Source
package and tree bytes are unchanged, and backend/extractor fingerprints include the central
workspace feature policy. No dependency version or Cargo.lock change was needed.

Final package **4,013,067 bytes**, SHA
`b7943c63d0997d88779e48ec74e1a97ef13d67e293f3250b2640340b034cdee4`.
Two fresh transport-final extractions agree on all five files. Extraction evidence SHA
`d66c2708b6bccf766da5db3c89d7f39c670ccfe99cd3912360ea135133c42c67`, extractor SHA
`52e675a0d87106f473302dffc74d32f2765cfa328824ce7f8facd884a2046f7a`.
Evidence: `runs/action-speed-transport-final-preservation.json`.

The caller-supplied calibration migration is complete: schema-1 catalogs load explicit
source paths, while schema-2 reports distinguish requested identities, observed reference
realization and unverified game legality. Six CLI tests pass, including caller-provided
Spark documents outside the old four-build corpus. The assembly benchmark also requires
an explicit caller problem. The legacy four-source library utility remains isolated
regression coverage, with its restrictive realization guards intact.

Next: check hosted CI, then continue B2/B3 breadth and general source/model work. A new individual-skill
port is not the next priority. Start with the source-preserving configuration seam described
in [breadth validation](breadth-validation.md#source-preserving-configuration-inspection), preserving
strict unsupported-mechanic decisions. Then produce the complete coverage inventory and a
concrete general build-model proposal for discussion before a major architecture change.

Publication: code **`caa56f4374da864acfb99cc19060ed356bd35105`** is pushed to main.
[Exact-code Windows/Linux CI run 34238329480](https://github.com/Azaril/poe-optimizer/actions/runs/34238329480)
is **in progress** on both platforms; hosted success is not yet claimed. Exact-commit check
snapshot: `runs/action-speed-main-check-runs.json`. The following publication-record commit
changes documentation only. Check the completed hosted outcome at the next resume point.

### Isolated action-timing release evidence

Windows x86-64, 32 available logical processors, three samples per mode/worker count,
700 ms target (actual 661–752 ms), with no concurrent builds or tests. The caller-loaded
[example](../examples/action-timing-search.json) generates 1,806 diagnostic selections:
eight classes, 23 named ascendancies plus no ascendancy, 129 allocations, seven support
loadouts and twenty equipment selections. This remains a bounded Mace working set; the five
new corpus builds are excluded by native admission and contribute no throughput claim.

| Workers | Fresh admission + calculation / second | Already-admitted calculation / second |
| --- | ---: | ---: |
| 1 | 38,868 | 4.81 million |
| 2 | 50,984 | 6.67 million |
| 4 | 97,889 | 13.28 million |
| 32 | 386,199 | 89.34 million |

All 24 samples and calibration checksums agree with preflight. Checksums consume all fourteen
metric statuses/values, thirteen receiving outputs, six movement outputs and six action-speed
fields, including optional MAX presence. Sixteen fresh document comparisons independently
check actor evidence. Timing intermediates absent from metrics are covered by separate
source/full-build parity, not by this benchmark checksum. Fresh admission includes allocations;
the already-admitted mode excludes actor assembly, XML, scoring, proposals and owned reports.
Neither is whole-optimizer throughput and neither uses a candidate-result cache.

All 24 complete CLI runs (typed/document × 1/2/4/32 workers × three repeats) produce identical
archives, statistics, verified selection, XML and data companions. Each consumes exactly
1,000 total evaluations: one baseline plus 999 search attempts including one fresh finalist
check, with zero evaluation failures. Typed median elapsed time is 352–377 ms; document
medians fall from 3,261 ms at one worker to 704 ms at 32. Dataset preparation alone is about
290 ms in this run, so the small typed search is dominated by setup. These budgets are
reproducibility checks, not the planned 5–30 minute optimizer-quality benchmarks.

The exported finalist matches fresh native and PoB results on all seven objective metrics:
153.92085991099998 selected hit DPS, 130% action speed, 170.3% movement speed, 212 Armour,
358 Evasion, 97 Energy Shield and 75% Fire Resistance. All constraints hold. XML SHA
`2262dc512808fbd9bc71f75236979818bc018c93ed8dd29008c1b3d27d772601`.

Reproduce the measurement harness with:

```powershell
cargo run --release --no-default-features --locked --example benchmark_assembly -- --problem examples/action-timing-search.json --sample-ms 700 --repeats 3 --jobs 1,2,4,32
```

Evidence: `runs/action-speed-benchmark-release.json`, `runs/action-speed-benchmark-summary.json`,
`runs/action-speed-release-search/summary.json` and `runs/action-speed-finalist-parity.json`.
Release CLI SHA `1f4b5fab1b271857c6730cad77f2924aebb63b22a847db21522dce1022a0806d`,
benchmark SHA `b592482ecb060cd1154361f0f87224374b5e3a98223d1581bf3654b5b8ef1c36`.
Source problem, implementation, template, dataset and extraction identities are retained in
the reports. Original source/golden preservation and the 34-document local-link audit pass.

## Body Armour and shared movement - implementation checkpoint

Implementation is complete for the bounded forms in the [operational guide](body-armour-movement.md).
Full integrated validation and isolated release measurements pass. Publication is recorded
below. General skill/build replacement remains unfinished.

Movement is prepared once from the same ordered actor queries as resources and receiving
defences. Item-local consumption precedes surviving source globals and generated armour
penalties. The penalty record preserves the item's source and a negated dynamic
IgnoreMovementPenalties condition; absent and explicit zero base penalties remain distinct.
The public metric is `100 * EffectiveMovementSpeedMod`, using definition schema 1, unit
`percent` and player scope. It is a ratio against baseline, not an increased-speed stat.

Full-build comparisons exposed a parser gap: original ModParser lowercases input, while
native actor/local-weapon literal matching had been case-sensitive. Matching now uses ASCII
case-insensitive literals, with original source bytes/captures retained. Exact item-formatting
keys still run first and remain case-sensitive. Source rejects condition suffixes on the
three special movement phrases; both native pipelines retain that rejection. No source
numeric expectation was weakened to make these cases pass.

| Area | Current evidence |
| --- | --- |
| Injected data | Schema **10**, `poe2-native-profiles-v10`, **19** sections and **35** verified source files. **402** armour bases include **114** admitted Body Armour definitions from 347 final body definitions. **347** actor templates, **83** exact formatting keys and **1,282 admitted / 3,476 excluded** passive views. Formula defaults, rounding, penalty mapping and grammar come from injected data; ActionSpeed must remain one until all consumers are represented. |
| Shared Rust engine | Both full actor entry points prepare movement after final attributes/conditions. Override zero, skipped first rounding, literal division, floor order, signed values and final effective rounding follow source. Global equipment order is Helmet/Body/Gloves/Boots; numerical receiving order is Helmet/Gloves/Boots/Body. **109 engine tests** pass, including **620** new cold/warm source comparisons. **4,860** varied four-slot actor preparations followed by **9,720** Spark/Mace calls allocate zero times. |
| Independent source data | **54 data tests**, **51 PoB unit tests** and **16 direct source tests** pass. New body/condition/formula oracles cover **2,736** cold/warm cases; mixed-case parser checks add **16** cold/warm pairs. ASCII-folded duplicate grammar templates are rejected, while exact formatting keys retain source precision behavior. |
| Import/native | Privately bound typed actors preserve source, generated and final global records. `local_armour` evidence is schema **2**, `movement` evidence schema **1**. Native Spark/Mace media versions are **6 / 8**, engine profiles `poe2-spark-body-movement-v6` / `poe2-mace-strike-body-movement-v10`. Native snapshots contain thirteen metrics (all thirteen finite for Spark; twelve finite and the existing unavailable average-hit metric for Mace). **116 import + 60 native tests** pass on the final code, including the complete joint-search matrix. Evidence: `runs/movement-adapter-all-tests.log`. |
| Full-build parity | **40 fresh native/PoB pairs + 10 exact native-export reimports** pass across both skills, four slots/removals, generated penalties, quality, local/global source composition, conditions, signed/fractional boundaries, INC/MORE/less, zero/57/31/117 overrides, floor/ignore flags, mixed case and source formatting. Six unsupported conditional-special-phrase cases reject. Fixtures are separate from original independent goldens. Evidence: `runs/movement-integration-final.log`. |
| CLI | Graph problem **10** / report **11**, scope `movement_native_search_v1`; older 7/8/9 graphs reject movement/body scope, including unselected supplied alternatives. Mutation 1–6 reject authored movement. **15** graph integration tests pass, including typed/document × one/four workers, locks, configurable constraints and custom injected penalty ranking with exact replayable exports/data companions. |
| Release search | **24** complete searches (typed/document × 1/2/4/32 workers × three repeats) agree on archives, ledger, finalist, exact XML and companion. Each uses 1,272 proposals/source admissions, 162 duplicates, 28 rejections, six rounds and **1,000** total attempts (baseline 1 + search 998 + fresh verification 1). No failures, late results or unavailable assessments. The finalist has DPS 122.17940598499999, fire resistance 75, ES 97, Armour 343, Evasion 231 and movement 118.8%, meeting the configurable 75/50/200/150/115 floors. Its exact export matches fresh native assessment and all six PoB metrics (maximum absolute difference `1.43e-14`). Evidence: `runs/movement-release-search/summary.json`, `runs/movement-finalist-parity.json`. This validates diagnostic reproducibility, not search quality, obtainable affixes or a global optimum. |
| Final validation | **658 workspace tests + 87 native-only CLI tests pass**, each through a complete successful all-target invocation. All **76** integration targets reconcile with Cargo metadata; nine ignored source-worker helpers are exercised by parent tests. Strict workspace/native-only all-target Clippy, formatting, five portable WASM libraries, runtime PoB/Lua dependency isolation, 32-document/347-link audit and diff checks pass. Final cross-agent review found no blockers. Evidence: `runs/movement-test-coverage.json`, `runs/movement-workspace-tests.log`, `runs/movement-native-only-tests.log`, `runs/movement-doc-audit.json`. Later changes are formatting, the extraction test name, restored original notes bytes and CRLF-safe test fixture edits; all three affected native test targets pass in full (19 tests). See the hosted Windows diagnosis below. |

Final package **4,007,917 bytes**, SHA
`0d1c8e4dca686a3d90c3d2928b4108ca8f5c1d186a0085d7edd4057881e3db50`.
Two fresh extractions (`runs/movement-final-extraction-a` and `-b`) agree across all five
output files and match the installed package. Evidence **4,074 bytes**, SHA
`6a2626aeb266d93e094562202af5fa81d06ad1b5c84ff0d71ff6a8c277bf1d84`;
extractor SHA `0bbe7dd9e9de5ba5a719acbbd281f015f61ae98f6b94174ec0591db049921f85`;
policy SHA `995239801ab9f4bd66658ea8818cd7c0b5e405d95c20be42821e4b673301f40d`.
The final preservation audit (`runs/movement-preservation-final.json`) verifies pinned
unmodified source, unchanged tree/full source snapshot, supplied originals, six independent
golden pairs, dependencies/workflow, all 329 previous actor rules, 77 formatting keys,
288 prior armour bases (apart from the explicit null penalty field), 1,270 prior passive
views, seven amulets and thirteen other sections.

### Hosted Windows test diagnosis and repair

Prior code `e42760d2` passed Linux CI, but Windows run `34221294167` failed in
`armour_bases_grammar_and_requirements_follow_selected_injected_data`. Git supplied CRLF
fixture text, so an LF-only multi-line replacement silently omitted `LevelReq: 1`; the
subsequent level-61 rejection was correct. Explicit CRLF input reproduced the same failure
locally (`runs/movement-windows-crlf-reproduction.log`). No numerical expectation changed.

The test now exercises LF and CRLF variants and asserts its unique source edit. Review
found and fixed the same assumption in two new movement test mutations (requirement
metadata and a rejected sockets line). All three complete affected targets pass: four
armour, eight native contract and seven movement tests, with targeted strict Clippy.
Evidence: `runs/movement-windows-crlf-final.log`, `runs/movement-crlf-native-tests.log`,
`runs/movement-prior-windows.log` and `runs/movement-prior-ci-jobs.json`. An incidental
encoding change in the large-notes test was restored to the original bytes. Production
source and release binaries were unchanged, so measured release evidence remains valid.
The new exact-code hosted run must still confirm Windows after publication.

### Body Armour/movement release measurements

[The assembly benchmark](../examples/benchmark_assembly.rs) accepts `--body-armour`.
The native-only release run on the AMD Ryzen 9 9950X3D uses **1,806** admitted selections,
129 allocations, eight classes, 23 named ascendancies plus no ascendancy, three attribute
options, seven support loadouts and twenty equipment selections. Four optional armour slots
rotate through supplied components and empty slots. The **1,518** distinct output checksums
consume all thirteen metric availability/value fields, all thirteen receiving outputs and
all six movement fields. Sixteen fresh full native document comparisons check metrics,
receiving and movement outside timing; independent PoB checks are recorded above.

Median attempts/second, three samples per worker count and mode:

| Workers | Fresh admission + calculation | Reused admitted actor + calculation |
| --- | ---: | ---: |
| 1 | 43,312 | 6,046,279 |
| 2 | 57,830 | 8,636,248 |
| 4 | 98,716 | 16,998,490 |
| 32 | 389,540 | 110,329,683 |

All 24 calibration/sample checksums match preflight. Ordinary samples last
1.070–1.294 seconds; the three 32-worker reused-actor samples hit the
100-million-attempt cap at 0.906–0.946 seconds. Fresh admission includes selection
cloning, structural/source resolution, actor execution, requirements and handle allocation/
destruction. Reused-actor timing excludes those costs. Both loops omit XML/JSON,
objective/proposal/archive work and exports, and use no evaluation-result cache. These
rates are not full optimizer throughput or evidence for general-build performance.

Setup takes 297.34 ms for data, 8.09 ms for the catalog, 1.73 ms for numerical components
and 46.26 ms for 1,806 initial admissions. Partial storage includes 221,120 compiled-actor
heap bytes, 8,478 local-armour heap bytes across thirteen components, 4,530 source XML
bytes, 1,821 prepared numerical component bytes and 792 bytes of scratch per worker.
Shared data, selections/maps, allocator metadata and thread stacks are excluded; these
figures are not peak memory.

Complete CLI medians in milliseconds, three separate-process runs per cell:

| Workers | Typed path | Fresh document path |
| --- | ---: | ---: |
| 1 | 368.34 | 3,045.97 |
| 2 | 352.19 | 1,805.28 |
| 4 | 353.74 | 1,126.79 |
| 32 | 347.59 | 676.06 |

CLI elapsed time includes source/data/catalog preparation and search, excluding final
report/export publication. Separate process medians are recorded in the raw summary.
Data setup dominates this small typed example; it does not establish optimizer scaling
or quality on general builds. All project builds/tests were finished before each isolated
measurement window. Runtime execution uses Rust/Rayon only.

Raw evidence: `runs/movement-benchmark-release.json`, `runs/movement-benchmark-summary.json`,
`runs/movement-release-search/summary.json` and `runs/movement-finalist-parity.json`.
Benchmark binary SHA `589c209e673e1fac115b421fe40de4e98f64f58dbbbf873e17d2af606b251a58`;
CLI binary SHA `a37056389d956082a858ed787685fefccd6f7f80b7b9d3356416a71eece2d32c`;
search implementation SHA `afe0533ee36bf97e3c4851ba006d9ab211d2a2d395d14438c6666ee4a6618f7e`;
finalist XML SHA `0229c476e87039dd4f79775559504d4ea7e4e4b832d6cf8331f5904e85610438`.

Publication: code **`56dcbc500ebdf3f2011663462c4c0a0a794aaabe`** is pushed to main.
[Exact-code Windows/Linux CI run 34228787060](https://github.com/Azaril/poe-optimizer/actions/runs/34228787060)
has **passed on Windows and Linux**. Refreshed jobs snapshot:
`runs/movement-main-ci-final-jobs.json`.
The following publication-record commit changes documentation only.

### Completed scope: shared action speed and action timing

This slice resolves ActionSpeed once after final attributes/conditions, then feeds
movement and both ordinary direct-action pipelines. All three consumers now share the result; the prior movement-only neutral default guard
has been replaced within the documented source-admission scope. The following source audit
and acceptance requirements are implemented and retained for future parity work.

1. Extract ActionSpeed INC, TemporalChainsActionSpeed INC, MinimumActionSpeed MAX,
   MaximumActionSpeedReduction MAX and UnaffectedBySlows FLAG plus the source grammar,
   flags/tags, TemporalChainsEffectCap (75) and ServerTickRate (`1 / 0.033`). Do not admit
   BASE/MORE/OVERRIDE ActionSpeed that the source function never queries. Popular minimum
   phrases carry GlobalEffect/unscalable metadata and require whole-source admission.
2. Extend ordered queries/programs with filtered positive-row sums and MAX returning
   `Option`. `ModStore.lua:228–238` filters each evaluated Tabulate row; clamping the final
   grouped sum changes semantics. MAX at 369–378 ignores zero/negative values and returns
   nil when no positive row matches. Preserve layer order and row-level cancellation.
3. Prepare a fixed, owner-bound ActionSpeed result with source min/max order and no added
   rounding. A shared direct-action timing helper must round skill INC × MORE speed to two
   decimals before ActionSpeed, then apply server-tick saturation. In `CalcOffence.lua:2980–3017`,
   CastRate is assigned again after selfCast multiplies by ActionSpeed, before Speed is
   capped at ServerTickRate × Repeats. The final CastRate therefore includes ActionSpeed
   and excludes the cap. Time (3049–3053) is zero when Speed is zero, otherwise its
   reciprocal; DPS (4576) consumes capped Speed. Both ordinary Mace and Spark are selfCast (`CalcActiveSkill.lua:635–643`).
   This checkpoint closes the prior high-rate cap gap in native kernels and their
   independent source harnesses.
4. Validate original cold/warm MAX/Tabulate/actionSpeedMod and the complete offence branch;
   include mixed-sign rows and layers, absent/zero/negative MAX, floor/ceiling conflicts,
   conditional flags, neutral preservation, zero/tiny/high speeds, tick-boundary neighbors,
   injected local-weapon/support speeds and all seven loadouts. Add complete PoB builds,
   exact export/reimports and changing actor/skill loops with zero measured allocations.

Per-level armour follows broader implicit/metadata admission and Ward consumers.
`Item.lua:2530–2556` retains local slopes; `GetArmourDataValue:2373–2379` adds a separately
rounded slope × **character level**, not item level or a rounded combined fixed/sloped sum.
The actual Fists of Stone source gloves (`gloves.lua:2039–2059`) are hidden and have multiple
unscalable implicit lines; the runeforged form also needs Ward receiver/recovery/hit-pool
semantics (`CalcDefence.lua:1348,1460,2054,3182,3625`). Retain rejection until the whole
producer and all required consumers are represented. Ailment, party, trigger, channel,
warcry, totem and reload producers likewise need complete dependency paths.

## Breadth of validation and data-driven build admission — next phase

Delivery order: immediately after the action-timing checkpoint and before another
individual-skill port. This phase addresses the user's concern that a small Spark/Mace
working set can hide structural assumptions. The end state remains the fully native,
injectable-data evaluator with optional PoB parity; no product scope has been narrowed.

Initial intake on 2026-09-08 preserved all **68,354 bytes** of the five-line input, SHA-256
`3e763f109adb27d48f2cf63a8a95aaea649e5336dcaf37959931725c29f6c745`.
All five share strings decode and run in the pinned PoB reference. They represent three
class/ascendancy combinations, minions, spear and quarterstaff actions, triggered/supporting
skills, weapon-set allocations, and multiple saved skill/item/tree sets. At initial intake, all five native
runs stopped at the XML literal attribute-whitespace compatibility guard. This first error is
not a full mechanic coverage inventory, and a successful reference run is not a legality
or Full DPS certification. Details and the checked-in intake index belong in
[breadth validation](breadth-validation.md); local raw evidence is
`runs/breadth-intake-20260908/manifest.json` and the per-build outputs beside it.

B1 intake tooling and the initial reproducible corpus are implemented: `scripts/intake-build-corpus.py` takes explicit corpus,
backend executable/data/options paths and budgets. Six regression tests pass. A real mixed
backend run preserves all five entries, five reference successes and five native first-error
rejections, with no changed inputs (`runs/breadth-validated-20260908/index.json`). An immutable
copy beside the indexed XML decouples this corpus from future edits to the user's working
file. The generic admission/coverage matrix and held-out corpus are still pending.
The first common native barrier was one multiline quest-reward string in active ConfigSet 1.
The shared source projection now resolves that XML-semantic mismatch without implementing
its game effects. The complete injected catalog and data-bound inspection are described in
[the configuration checkpoint](#injected-configuration-definitions--validation-checkpoint).

- [x] **B1 — caller-configured corpus and provenance.** Load a versioned manifest or
  line-delimited import file supplied by the caller. Preserve exact inputs, hashes,
  source/data/backend versions, active and inactive sets, selected actor/action/part,
  configuration and encounter assumptions. Decode each entry independently and report
  per-entry errors without discarding the rest. Capture fresh PoB outputs independently
  of cached XML statistics. Do not silently migrate tree versions or skill selections.
- [ ] **B2 — breadth and coverage inventory.** Resolve every represented skill, support,
  item modifier, passive allocation, actor, condition and dependency as supported,
  unsupported or unresolved. Separate container decoding, XML semantic compatibility,
  intended-versus-realized build identity, legality and numerical parity. Keep first-failure
  admission results distinct from the complete static inventory. Summarize by mechanism and
  whole-build family, including exclusion counts; do not count a shared archetype or an
  alternate saved set as an independent held-out build. Investigate the existing three
  unresolved entries in line 1 against the pinned reference before inventing replacements.
  The configuration catalog, root-container, complete skill identity and three-label audits are now recorded in
  [breadth validation](breadth-validation.md). The subsequent
  [mechanism inventory](breadth-mechanism-inventory.md) covers all saved equipment/passive
  sets and fresh MAIN grants, actors, support links, allocation providers and conditional
  records. The observed static capability comparison now distinguishes native query primitives from
  source producers. Shared native FLAG/GetCondition now has actual-source parity and 15
  captured query-closure replays, with four unresolved proposals retained; this does not
  close complete actor dependencies. The ordered item-loading phase adds the complete
  injected item catalog and source-bound partial execution reports, with native parser and
  assembly dependencies explicit. Full dependency closure, inactive-selection inventory
  and whole-build admission remain open.
- [ ] **B3 — general build-input seam review.** Audit hard-coded fixture inputs, closed
  Spark/Mace admission, skill IDs, level restrictions and implicit player-only assumptions.
  Normal evaluate/search already load caller files, but that alone does not establish
  generic admission. The shipped developer `search-calibration` command now requires a
  caller-supplied catalog; embedded production payloads have been removed. The old four-fixture
  PoB materializer now exists only in integration-test support, outside the production API. Its schema-2
  report separates exact requested source from observed realization and unverified legality.
  The assembly benchmark likewise requires an explicit caller problem path.
  Propose a normalized build model for item/skill instances, grants, actor relationships,
  conditions and shared calculation dependencies. The native engine receives data and typed
  operations; fixture names, source hashes or character identities must never select
  special calculation code. Discuss a significant architecture change before implementing it.
- [ ] **B4 — independent differential corpus.** Keep small kernel fixtures for diagnosis
  and add complete development and held-out builds across attacks, spells, minions, damage
  over time, conversion, triggers, cooldown/repeat/channel behavior, defence/recovery,
  reservations and supporting-skill interactions. Include bossing and mapping with explicit
  assumptions. The five supplied builds seed the inventory; they do not cover all families.
  Use whole-build holdouts chosen before porting a mechanic, source-derived tolerances and
  intermediate actor/action evidence. Never regenerate expectations from the native result
  or simplify an original build to make it pass. Store derived perturbations separately.
- [ ] **B5 — identity, legality and interaction regressions.** Exercise import/export/reimport,
  active-set changes, required 1..N skills/items, locks, quality/level changes and interacting
  support/equipment/passive deltas. Check ordinary and ascendancy paths from their separate
  roots with independent point budgets; starts are implicit and cannot spend the other's
  points. Preserve weapon-set and exceptional-allocation provenance. Report source-missing
  node targets separately from semantic ascendancy roots, item-enabled radius allocations
  and candidate paths disconnected by a mutation. Fresh realized output must match the intended candidate.
- [ ] **B6 — coverage-led delivery and performance gates.** Rank shared dependencies by the
  number and variety of corpus builds they unblock, rather than adding isolated skill
  profiles. Track admitted/unsupported builds and available metrics alongside parity results.
  Require full-build differential checks for each newly claimed capability; exclusions stay
  visible and do not count as passing parity. Measure native preparation, direct parallel
  evaluation and complete search separately on realistic admitted builds. Reference worker
  costs are not native costs; no Lua subprocess or hidden PoB fallback belongs in the hot path.

Exit from the initial breadth phase requires a reproducible corpus runner/index, an honest
coverage matrix with independently captured reference results, and an agreed prioritized
shared-pipeline plan. It does **not** require pretending all five builds already work natively.
Full replacement completion later requires broad held-out full-build parity, including
reported unavailable/nonfinite outcomes, exact realization, and useful parallel performance.

The [tree investigation](tree-topology-investigation.md#ascendancy-components-and-separate-point-budgets)
confirms all 23 catalogued ascendancies have roots. The 14 existing warnings concern IDs
absent from source, not the ordinary/ascendancy point split. Their individual identities
remain unproven; no synthetic reconnection is planned. The corpus's six ordinary singleton
notables have observed From Nothing radius-provider links; their legality must retain that
item dependency and a separate point-budget check.

## Local armour equipment and rating objectives - implementation checkpoint

Implementation, integrated local validation and release measurements are complete for
this bounded scope. The [operational guide](local-armour.md) describes admitted forms,
configuration data, local/global composition, metric contracts and unsupported mechanics.

The shared engine consumes eligible item BASE/INC records, applies quality separately and
rounds local ratings. Surviving globals contribute exactly once in source slot order;
receiving aggregation adds each slot separately before the global term. Energy-Shield BASE
and INC pair orders remain distinct. Prepared local components bind their dataset and slot;
prepared actors retain numerical outputs without XML or borrowed equipment references.

The item-formatting seam fixes a full-build discrepancy: original `ItemTools.applyRange`
formats `+17.5 to Evasion Rating` as `+18` before ModParser, producing Evasion 47 where raw
native parsing gave 46. Injected exact case-sensitive keys govern this step, including
weapon and amulet lines. Missing keys preserve decimals; configuration remains raw. Raw
and effective captures are separate evidence and exports keep original bytes. Custom
formatting that emits decimal text for an integer-only capture is rejected, preserving
the admitted source grammar. Both cases have retained regressions.

| Area | Current evidence |
| --- | --- |
| Injected data | Schema **9**, `poe2-native-profiles-v9`; **18** sections, **34** verified source files, **288** complete fixed bases (112 Helmet / 88 Gloves / 88 Boots) from 649 final definitions. Shape-based extraction excludes 361 unsupported definitions. **329** actor grammar templates and **77** exact item-format keys. No evaluator base-name allowlist or embedded patch-specific base values. |
| Shared Rust engine | Immutable `PreparedArmour`, borrowed `ArmourSlots`, complete actor preparation and separate per-slot receiver inputs. **101 engine tests** pass, including **1,694** original local/getter/receiver comparisons and **3,360** original numeric formatter comparisons across cold/warm modes. **3,240** changing three-slot actor + Spark/Mace loops allocate zero times; per-worker scratch remains **792 bytes**. |
| Import/native | **110 import + 52 native tests** pass. Shared source classification applies item formatting, local consumption, remaining globals and final requirements. Private catalogue/data/scenario bindings, exact source/raw/effective evidence and fresh realization reject tampering. Varied typed equipment calculations retain the allocation-free path; preparation costs are measured separately. |
| Independent source data | **49 data tests**, **48 PoB unit tests** and **14 direct source tests** pass. Original parser checks cover **3,750** cold/warm inputs; **5,760** original base/local-armour cases include actual applyRange before ModParser. Formatting policy checks cover **616** source observations and **28** explicit case/sign/fallback examples. |
| Full-build parity | **33 fresh native/PoB pairs + 11 native-export PoB reimports** pass, covering signed/fractional boundaries, quality, local/global pairs, condition/order effects, requirements and configuration/gear/passive composition. New Spark/Mace fixtures are separate from original goldens. The exact release-search finalist also matches fresh PoB on all five objective/constraint metrics (maximum absolute difference `1.43e-14`). Evidence: `runs/armour-cli-remaining-final.log`, `runs/armour-finalist-parity.json`, `runs/armour-finalist-pob.json`. |
| CLI contracts | Graph problem **9** / report **10**, scope `local_armour_native_search_v1`. Graph 7/8 reject armour-bearing templates and supplied alternatives, including unselected alternatives. Earlier receiving gates and mutation 1–6 scope remain. **11** graph integration tests pass, including typed/document × one/four workers with identical archives, ledgers, exact XML and companions. Player `armour` / `evasion` use definition schema 1 and `rating_points`; both backends reject selected-minion requests for these definitions. Native snapshots contain twelve metrics. |
| Integrated validation | **618 unique workspace tests + 82 native-only CLI tests pass** across integrated/focused runs. All **73** direct integration targets are reconciled against Cargo metadata; nine ignored source-worker helpers are exercised by parents. The initial full workspace invocation stopped at a stale legacy Mace media-version expectation (6 to 7); the corrected complete target, all remaining CLI targets and full final library invocation pass with numeric assertions unchanged. This is reconciled coverage, not one all-green workspace invocation. Evidence: `runs/armour-test-coverage.json`, `runs/armour-libraries-final.log`, `runs/armour-cli-remaining-final.log`, `runs/armour-native-only-tests-final.log`. |
| Additional checks | Strict workspace/native-only all-target Clippy, formatting, five portable WASM libraries, runtime PoB/Lua dependency isolation, documentation links and diff whitespace pass. Final read-only cross-agent review found no blockers. No unresolved local failures remain. |
| Release search | **24** complete native-only searches (typed/document × 1/2/4/32 workers × three repeats) agree on archives, ledger, finalist, exact XML and data companion. Each uses 1,496 proposals/source admissions, 172 duplicates, 226 rejections, seven rounds and exactly **1,000** full attempts (baseline 1 + search 998 + fresh verification 1). No failures, late results or unavailable assessments; termination is the evaluation budget. The finalist has DPS 122.17940598499999, fire resistance 75, ES 177, Armour 212 and Evasion 231, satisfying the configurable 75/50/200/150 floors. This establishes reproducibility for the diagnostic example, not optimizer quality, affix legality or a global optimum. |

Final package **3,865,166 bytes**, SHA
`afe99d3e5943f093487d0c885ee1b8facfcfbc80f7bda65d641d8aec0bfce384`.
Two fresh source extractions agree byte-for-byte across all five output files
(`runs/armour-complete-extraction-a` and `-b`); installed data matches exactly.
Evidence SHA `489f3d59d04139227a808887e677b432c820d478f02f541b86daaaef5be447e6`;
extractor SHA `777cb0ad19ce4833a9abfc955a3afb77b02b4da5220569969587a6ef26e353e1`;
policy SHA `06eaaacc41c1fb2954fff2041b215bee9d9f24c54cfedae0794265223342d960`.
The preservation audit (`runs/armour-preservation-final.json`) confirms clean pinned source,
unchanged full source snapshot/tree, original exports and six independent golden pairs,
dependency/workflow content, fifteen prior non-actor numeric sections, original 320 actor
rules/constants, seven amulets and all **1,270** passive views/**3,488** exclusions.

Publication: code **`e42760d233e37d75fcc04b07e6a30634fb7fdae9`** is pushed to main.
[Exact-code Windows/Linux CI run 34221294167](https://github.com/Azaril/poe-optimizer/actions/runs/34221294167)
has **passed on Linux** and **failed on Windows** in an LF-only test fixture mutation.
The current Body Armour/movement checkpoint reproduces and fixes the CRLF case; see its
hosted Windows diagnosis above. Refreshed snapshots: `runs/movement-prior-ci.json` and
`runs/movement-prior-ci-jobs.json`. Prior snapshots:
`runs/armour-main-ci.json`, `runs/armour-main-ci-jobs.json`. The following publication-record
commit changes documentation only.

### Local armour release measurements

[The assembly benchmark](../examples/benchmark_assembly.rs) accepts `--local-armour`.
The native-only release run on the AMD Ryzen 9 9950X3D uses **1,806** admitted selections,
129 distinct allocations, eight classes, 23 named ascendancies plus no ascendancy, three
attribute options, seven support loadouts and twenty equipment selections. Optional armour
slots rotate through supplied items and empty slots. Its **1,518** distinct checksums consume
all twelve metric availability/value fields and all thirteen receiving outputs. Sixteen
fresh full native document comparisons check metrics and receiving values outside timing;
independent PoB evidence comes from the source/full-build tests above.

Median attempts/second, three samples per worker count and mode:

| Workers | Fresh admission + calculation | Reused admitted actor + calculation |
| --- | ---: | ---: |
| 1 | 46,592 | 5,958,199 |
| 2 | 55,424 | 8,819,737 |
| 4 | 85,377 | 17,912,117 |
| 32 | 274,462 | 119,521,365 |

All 24 sample/calibration checksums agree. Samples last 1.100–1.244 seconds except
32-worker reused-actor samples, which hit the 100-million-attempt cap at 0.824–0.843 seconds.
Fresh admission includes selection cloning, structural/source resolution, actor execution,
requirements and handle allocation/destruction. Reused-actor timing excludes that work.
Both timed loops omit XML/JSON, objective/proposal/archive work and exports. Neither uses
an evaluation-result cache. These rates are not full optimizer throughput.

Data setup takes 278.65 ms, catalog 14.29 ms, numerical components 3.58 ms and 1,806 initial
admissions 42.74 ms. Partial storage estimates include 218,624 compiled-actor heap bytes,
5,578 local-armour heap bytes across nine armour components, 4,302 source XML bytes,
1,755 prepared numerical component bytes and 792 bytes of scratch per worker. They exclude
shared data, selection/map/allocator overhead and thread stacks; they are not peak memory.

Complete CLI medians in milliseconds (three separate-process runs per cell):

| Workers | Typed path | Fresh document path |
| --- | ---: | ---: |
| 1 | 353.56 | 3,461.81 |
| 2 | 353.13 | 2,051.35 |
| 4 | 344.30 | 1,252.69 |
| 32 | 347.56 | 672.86 |

CLI elapsed time includes source/data/catalog preparation and search, excluding final
report/export publication. Process medians include those remaining costs. Dataset setup
dominates this small typed example; the result does not establish scaling or search quality
on general builds. All builds/tests were stopped during each release measurement window.

Raw evidence: `runs/armour-benchmark-release.json`, `runs/armour-benchmark-summary.{json,md}`
and `runs/armour-release-search/summary.json`. Benchmark binary SHA
`0076738fdfefd8fee299eb3cc3b66c6b8620a1ed933e35448cf4c830111f0c86`;
CLI binary SHA `d8032a634dc2879345f6f47ac977fead63d5a6d4bd6caf4f695f466634d4bd4e`;
search implementation SHA `6f11701084844c3c94e9137555f8ad50dbe0e125bafffee571c983f4ba219368`.
Finalist XML SHA `815ff4994a33465084ed4b93bf7942ef278281fed31e797fabcda32afe7844cc`.

### Historical next-pipeline audit (completed by the checkpoint above)

Source audit recommended **Body Armour and shared movement calculation**. Add injected
body base movement penalties, the fourth armour slot and exact generated movement records
before exposing a movement objective metric. Do not admit the slot while discarding its
penalty. `Item.lua:2561–2562` emits MovementSpeed BASE `-movementPenalty` with negated
IgnoreMovementPenalties condition after local consumption; preserve its source/order.
`CalcDefence.lua:1911–1915` computes the modifier from BASE/INC/MORE or OVERRIDE, rounds
at three decimals, applies the optional cannot-be-below-base floor and multiplies by
ActionSpeed. Source condition resolution is in `ModStore.lua:409–413`. The existing strict
profiles can establish ActionSpeed 1 while rejecting unsupported action/party/skill
movement effects; extract that default from source rather than silently assuming it.

The source text inventory suggests 114 complete fixed/unhidden/no-implicit/no-Ward body
bases among 347 final names, with movement penalty ratios .03/.04/.05. Confirm counts by
executing original source before publication. New source grammar/flags, attribute-conditioned
ignore/floor cases, source query order, full Spark/Mace PoB/export parity, slot removal,
custom-data ranking, locks and allocation-free varied actor loops need independent tests.
No product answer blocks this continuation.

Per-level local Evasion/ES is a following extension. `Item.lua:2530–2531,2554–2555` keeps
unrounded local slopes; `GetArmourDataValue` at 2373–2379 adds the separately rounded slope
multiplied by **character level**. Do not use item level or round a combined fixed/sloped
sum. Actual coefficients are modifier records, not a generic per-level base field. Current
base-file producers are hidden Fists of Stone variants (`gloves.lua:2038–2059`) with special
multi-line `{unscalable}` implicits; one also needs Ward. Keep these bases excluded until
whole special-base/implicit policy and downstream consumers are represented.


## Shared receiving defences and resistances - implementation checkpoint

Implementation, integrated local validation and release measurements are complete for
this bounded scope. [Operational guide](receiving-defences.md) documents the new data,
API and CLI contracts. Code is published; exact-code Windows/Linux CI passes.

| Area | Current evidence |
| --- | --- |
| Injected data | Package schema **8**, `poe2-native-profiles-v8`, **3,561,244 bytes**, SHA `fdd924e0449d06c338df95cf8e309986abf07a73e3f2f0131acf222d5d24ae30`. Sixteen sections include ordered receiving query groups. **1,270 admitted / 3,488 excluded** effective passive views, **320** grammar templates and **seven** amulet bases. Tree schema 3 and its bytes remain unchanged. |
| Source evidence | Independent original-parser/ProcessStats/amulet/query checks pass: all admitted views and **3,642 authored parser inputs** cold/warm. New receiver source tests cover **1,178** cold/warm cases (1,104 boundary/group/signed-zero, 38 condition/order/quest, 36 injected-rule cases), comparing fresh DB and compiled programs against original Lua branches. |
| Shared engine | `prepare_actor` / `evaluate_actor` combine resource and receiving stages from the same ordered query object. Prepared fixed output binds data owner, character and receiving scenario, retaining no XML or source database. **95 engine tests** and scoped Clippy pass; **3,000** changing actor/receiver preparations followed by both skill kernels allocate zero times in counted loops. Scratch is **792 bytes**. Raw resource compatibility cannot silently consume receiving records or authorize changed defensive scalars. |
| Import/native | All **102 import + 47 native tests** pass across full and focused runs. Lunar/Pearlescent level boundaries, source Global markers, condition/pair parsing, selected passive migration, injected rules/quests/caps, typed/document equality, private scenario/data binding and receiving-evidence tampering are checked. One stale scalar-evidence tamper test was migrated to the ordered actor path and its full target rerun. |
| Full-build parity | **27 fresh native/PoB pairs and 10 native-export PoB reimports** pass, covering source groups, conditions, signed/fractional boundaries, configuration/gear/passive composition and formatting preservation. Two new source fixtures are separate from original goldens. The final diagnostic fixtures pass again in the integrated run. Evidence: `runs/receiving-build-parity.log`, `runs/receiving-workspace-tests.log`. A separate fresh PoB evaluation of the exact release-search finalist also matches all three objective/constraint metrics (`runs/receiving-finalist-pob.json`). |
| CLI | Graph problem **8** / report **9** adds authored receiving scope. Graph 7 and mutation 1–6 retain their authored scope; migrated passive records still work. **9 search integration tests** pass, including typed/document × one/four workers, identical archives/ledgers/XML/companions, new scope gates and the existing locks/output/budget regressions. An initially infeasible example seed was corrected; no evaluator assertion was relaxed. Evidence: `runs/receiving-search-cli-final.log`. |
| Extraction and preservation | Two fresh extractions agree byte-for-byte across package/tree/evidence; three extraction CLI checks pass. Evidence SHA `d5370031f67a5a59562473d2458d434928147a5b20fbdaa383463c6bf4f864e9`; extractor SHA `943c9cf9924552a82e167f6d63b6e9891421572693498d1fe217b231b3b58747`. Source pin/clean submodule, full snapshot, six independent golden pairs, original inputs, dependencies and eleven existing numerical sections are preserved. All 1,142 prior views are semantically preserved; exactly 36 defensive scalar records migrate once, with 128 newly admitted views. Original 116 grammar rules, actor constants/precision/quests and five amulets are unchanged. Evidence: `runs/receiving-preservation-final.json`. |
| Integrated validation | **580 unique workspace tests + 79 native-only CLI tests pass** across integrated and focused runs; nine source-worker helpers are ignored directly and exercised by parents. The initial full invocation used an old custom-resistance test editing a removed scalar field; the corrected complete target and all remaining CLI targets pass, as does the full library invocation. Its signed-value/feasibility/export assertions are preserved. Coverage is assembled explicitly, not presented as one all-green workspace invocation. Every integration target is reconciled in `runs/receiving-test-coverage.json`. Strict workspace/native-only Clippy, final benchmark Clippy, formatting, five WASM libraries, runtime dependency isolation and 31-document link checks pass. No unresolved local failures remain. |
| Release search | All **24** complete searches (typed/document × 1/2/4/32 workers × three repeats) agree on archives, ledger, finalist, XML and data companion. They use 3,480 proposals/admissions, 759 duplicates, 1,723 rejections and exactly 1,000 full attempts (baseline 1 + search 998 + fresh verification 1), with no failures/late results; termination is the evaluation budget. The diagnostic finalist has DPS 151.96940651249997, fire resistance 75 and ES 52, satisfying the example's 75/50 floors. This is reproducibility, not optimizer-quality, obtainable-affix or global-optimum evidence. |

Publication: code **`f1d8a1c404a8c3dc77b11d7ff4410a241b0d902b`** is pushed to main.
[Exact-code Windows/Linux CI run 34214745950](https://github.com/Azaril/poe-optimizer/actions/runs/34214745950)
has **passed on Windows and Linux**. Refreshed snapshot: `runs/armour-receiving-ci.json`.
The following publication-record commit changes documentation only.

### Receiving release measurements

[The assembly benchmark](../examples/benchmark_assembly.rs) now accepts `--receiving-defence`.
The native-only release run on the AMD Ryzen 9 9950X3D uses 1,806 admitted selections,
129 allocations, eight classes, 23 named ascendancies, three attribute options, seven support
loadouts and ten equipment selections. Its 1,318 distinct checksums consume all metric
availability/value fields plus all thirteen receiving outputs. Sixteen fresh complete native
document comparisons check both metrics and receiving diagnostics outside timed loops.
Independent PoB parity is supplied by the separate source and full-build suites above.

Median attempts/second, three samples per worker count and mode:

| Workers | Fresh admission + calculation | Reused admitted actor + calculation |
| --- | ---: | ---: |
| 1 | 52,903 | 6,530,000 |
| 2 | 62,020 | 9,350,000 |
| 4 | 96,893 | 19,309,000 |
| 32 | 437,760 | 129,215,000 |

All 24 samples and calibration checksums agree. Samples last 1.173–1.417 seconds except
32-worker reused-actor samples, which hit the 100-million-attempt cap at 0.773–0.784 seconds.
Fresh admission includes cloning, source/tree resolution, actor/receiver execution,
requirements and handle allocation/destruction. Reused-actor measurement excludes that
preparation. Both include full checksum consumption and Rayon; neither includes XML/JSON,
objective/proposal/archive work, final exports or a candidate-result cache.

Data setup takes 240.50 ms, catalog 6.14 ms, numerical components 1.18 ms and 1,806 initial
admissions 37.36 ms. Partial storage estimates are 218,624 actor-owned bytes, 3,575 source
XML bytes, 1,550 numerical component bytes and 792 bytes of scratch per worker. These exclude
shared data, selection/map/allocator overhead and thread stacks; they are not peak memory.
Raw evidence: `runs/receiving-benchmark-release.json`, summaries `runs/receiving-benchmark-summary.{json,md}`.
Benchmark binary SHA `87f6d662cf3831fa1864c73a8927c82c9c68b2efa6feb375ddf4011af03325d1`.

The separate whole-CLI release runs include source/data/catalog setup and search, and
exclude final report publication. Median milliseconds for typed/document modes respectively:

| Workers | Typed | Document |
| --- | ---: | ---: |
| 1 | 340.56 | 2,268.28 |
| 2 | 312.27 | 1,420.08 |
| 4 | 315.22 | 903.32 |
| 32 | 324.89 | 541.22 |

These small runs show that setup and search work can dominate inexpensive typed calculation;
additional workers need not improve overall runtime. Measure realistic 5–30 minute workloads
and optimizer quality separately. No other builds/tests/benchmarks ran during either timing
window. The benchmark and whole CLI have identical native/data identities.

CLI binary SHA `0dc1e802ae5f8ea39e49bb1db284758dcd42e7c173f99c24ca8efa467af8e3a6`;
search implementation SHA `16fda8e5f23baeafc4ac1f14d44534d7bc8d091be91e65c04d65bcc70359e9e9`;
final XML SHA `f8dc2c5ee21f5e39f68dce88d4b8de81e92ae0b053a2f2de7ef18014fc5d8d80`.
Evidence: `runs/receiving-release-search/summary.json` (SHA
`4161506cb94225122f68f5554a9881640d4a7f0642ce05e02180fb9bf53dd73f`), with verification
script `runs/verify-receiving-release.py` and the per-run reports/XML/companions.

### Next local-armour stage: source audit and acceptance

A read-only audit identifies normal/rare Helmets, Gloves and Boots with fixed
Armour/Evasion/Energy-Shield bases as the next useful equipment scope. Extract eligible
bases by complete supported source shape, including requirements, rather than embedding
base names in Rust. Their pinned base files lack movement penalties and per-level bases;
body armour already introduces movement effects and needs its downstream consumer.

- Keep prepared local armour as privately data-bound, per-slot numerical components.
  Do not flatten rounded item defences into ordinary global actor BASE records: the
  receiver queries slot contributions separately before global contributions.
- Follow literal `Item.lua:2384` local consumption and `:2522` calculation: individual and
  paired defence records, Defences INC, separate multiplicative quality and local rounding.
  Local negative results are not clamped; final receiver totals are. Global and conditional
  records survive local consumption. Increased maximum ES and increased ES have different
  source tags even when their English wording looks similar.
- Extend ordered equipment assembly to Weapon 1, Helmet, Gloves, Boots, Amulet, preserving
  the relative order in `ItemsTab.lua:32` / `CalcSetup.lua:1270`. Requirements still use the
  final shared actor; base requirements, explicit LevelReq and item level remain distinct.
- Initially reject per-level defence modifiers or implement their separate rounding:
  `Item.lua:2373,2554` scales the fractional local coefficient by character level and rounds
  separately from the fixed local defence. Do not use item level. Body movement penalties,
  Ward, block, special quality, slot-specific conditions, conversions and granted skills
  require complete consumers before admission.
- Compare original local outputs and `CalcDefence.lua:1365,1431` slot/global receiver paths.
  Full fixtures should cover bare/single/three-slot gear, pure/hybrid bases, normal/rare,
  quality 0/intermediate/20, local/global/conditional duplicate combinations, signed and
  fractional boundaries, add/remove/replacement, requirement locks and injected values.
  Compare existing PoB snapshot `*On<slot>`, `Gear:*` and final ratings, respecting their
  positive-only diagnostic branches, then exact exports, tampering and parallel search.

This is the next part of the existing equipment/native replacement plan. Movement,
reservation, arbitrary skills/supporting actors, minions and full-game parity remain
required later work. Armour/Evasion objective metrics need explicit unit/availability
contracts in both backends before they can be requested by users.

## Normalized passive and equipment source assembly - completed local checkpoint

Implementation and local regression/performance checks are complete for the bounded scope.
Code is published; exact-code Windows/Linux CI passes. Read [the operational guide](passive-equipment-assembly.md)
for schemas, APIs, command examples and limits.

| Area | Current evidence |
| --- | --- |
| Injected data | Package **schema 7**, `poe2-native-profiles-v7`, **3,428,384 bytes**, SHA-256 `bbea2a7b0eb2e6c9334a4b7cfccaff61ad57a41010949253ff540f61e535b10d`. Tree **schema 3**, **2,681,932 bytes**, SHA-256 `31cac8a09de2babc34e450d0caf975c45aca3caf222853d863dcad607c6f8779`. All 4,109 ordinary physical nodes and 4,758 effective source views are represented: **1,142 admitted**, **3,616 explicitly excluded**. The physical partial tree retains 4,141 records and excludes 773. Five source-derived amulet bases join the existing weapon/actor configuration. |
| Extraction and source oracle | Final fresh extractions `runs/passive-final-extraction-f` and `-g` reproduce package/tree/evidence exactly. Evidence SHA `be7a3b5847f281e5214be7e31b6152ce2a46656bdb8749c3ef4be50b3b0bd8a4`; extractor SHA `e94191f4974c710af5f2ef129f439b45c00cefdc227cb83aabb4ddfcc9709fdf`. **41 data tests, 11 independent data oracle tests, three new extraction guards and three extraction CLI tests pass**. Cold/warm original `PassiveTree.ProcessStats` compares every admitted view; selected replacement-option fields and jewellery records are fully checked. Final CLI reproduction covers 15 sections and 29 direct consumed source files. |
| Native actor assembly | Immutable actor programs compile once per source component and use borrowed fragments in one local layer with reusable scratch. **90 engine tests**, **1,280 cold/warm compiled-program comparisons** and **2,160 varied class/gear/passive actor-plus-skill zero-allocation loops pass**. Configuration, Weapon 1, Amulet and passive assembly preserve source order; complete passive BASE/INC records are admitted, while order-sensitive passive operations remain excluded. |
| Import and admission | Source-neutral actor grammar is shared by configuration and equipment. Local weapon records are consumed separately from global actor lines. Lazy `ControlledBuildCatalog` and private admitted handles retain exact item IDs/source, physical/effective passive keys, explicit attribute options, shared actor requirements and compiled-data binding. Full import suite **97 tests passes**, including early source/seed passive-count bounds. No Cartesian alternatives, XML hashes or result table is constructed. |
| Native boundary | `PreparedBuildCandidates` evaluates Spark/Mace from the same admitted actor used for requirements. Twelve varied joint combinations match complete native documents. **10,000 repeated pure measurements allocate zero times**; foreign handles and tampered tree/equipment/resource/context/export evidence reject. Profile IDs are `poe2-spark-passive-equipment-v3` and `poe2-mace-strike-passive-equipment-v7`; profile media are 3/5 and tree media is 3. |
| Full-build parity | **35 fresh complete native/PoB pairs and 18 exact native-export PoB reimports pass** across two new suites. They cover connected attribute paths, repeated effective attribute IDs, supported notables, class replacements, jewellery, rare weapon local/global composition and both skills. Native export re-evaluation also agrees. Evidence: `runs/assembly-passive-parity.log`, `runs/assembly-equipment-parity.log`. These are additional to the preserved legacy/golden suites. |
| CLI/search | Native-only `search-build` uses problem **7** / report **8**, caller constraints and attribute locks, bounded graph proposals, repaired required items/passives/skills and class/ascendancy restarts. Compact scheduler identity keys release old prepared candidates. Five seed and six integration tests pass for typed/document × one/four workers, exact archives/ledger/XML/companions, three-attempt verification, repaired two-item locks, impossible LevelReq with zero full calculations and output collision/alias preservation and failed imported baseline isolation. Seed repair is a heuristic, not an infeasibility proof. |
| Integrated validation | **556 unique workspace tests and 76 native-only CLI tests pass across integrated and focused runs**, with nine unique source-worker helpers ignored directly and exercised by parents. Initial broad runs found two stale expectations: the tree artifact's former 1 MiB test cap and a version-2 tree diagnostic assertion. Both were corrected, and their complete suites plus remaining targets pass. This records assembled coverage rather than claiming a single all-green workspace invocation. Full workspace/native-only Clippy, formatting, five WASM libraries, runtime dependency isolation and documentation links pass. Full import/native final run is **141/141**. Evidence: `runs/assembly-test-coverage.json` and named raw logs. |
| Preservation | Six independent XML/reference pairs, supplied originals, full snapshot SHA `68445629df3af8bb934aad91f5e6b457f8fe7e478b67e306493ee9a017d171f7`, source inventory/pin and dependency versions are unchanged. All eleven original nonpassive numerical sections and twenty legacy passive effects are preserved. PoB submodule is clean. Evidence: `runs/assembly-preservation-final.json`. |
| Publication | Code **`a2586a89a8160ae0b31d42d23ed921cb3b5823b2`** is pushed to main. [Windows/Linux CI run 34208805533](https://github.com/Azaril/poe-optimizer/actions/runs/34208805533) has **passed on Windows and Linux**. Refreshed snapshots: `runs/ci-34208805533-current.json`, `runs/ci-34208805533-jobs.json`. The following publication-record commit changes documentation only. |

The lazy generalized PoB realization adapter remains unimplemented: the new search command
is explicitly native-only, while full-document `evaluate --backend pob`, independent source
oracles and complete-build parity tests provide the reference. Legacy `search-experimental`
still supports both backends. Full PoB parity, supporting skills/minions, receiving defences,
reservation, unrestricted equipment and practical optimizer quality remain unfinished.

### Next receiving-defence slice: source findings

A bounded read-only review supports one shared player receiving-defence/resistance stage
before expanding equipment slots. Start with global Armour/Evasion/EnergyShield BASE/INC,
the actual `ArmourAndEvasion`/`Defences` query groups, and individual/elemental player
resistance BASE/INC. Preserve complete paired/all-resistance parser expansions and selected
attribute conditions. Keep MORE/OVERRIDE, maximum-resistance modifiers, conversion receivers,
Ward and slot-dependent forms rejected until their receiving stages are complete.

- This PoE2 pin gives Strength to Life, Dexterity to Accuracy and Intelligence to Mana;
  **there is no inherent Dexterity-to-Evasion or Intelligence-to-ES multiplier**.
  Source: `Modules/CalcPerform.lua:495–520`. Do not import PoE1 attribute assumptions.
- `ModParser.lua:2562` emits an explicit `Global` marker for increased maximum ES.
  Preserve it through item-local consumption (`Classes/Item.lua:2384–2395`); it is
  nonrestricting metadata in global ModDB evaluation, rather than permission to discard
  it before local/global classification (`Classes/ModStore.lua:490`).
- Receiving Armour/Evasion query individual, ArmourAndEvasion and Defences; ES queries
  EnergyShield and Defences. Apply the source BASE/INC order, final rounding and nonnegative
  clamp (`Modules/CalcDefence.lua:1339,1430`). Do not expand `ArmourAndEnergyShield` as two
  global stats: its explicit pinned consumer is the local armour stage.
- Resistance uses BASE times nonnegative INC factor, truncates toward zero, then applies
  separately truncated floor/cap; preserve elemental/individual query order and zero chaos
  penalty (`Modules/CalcDefence.lua:925`, `Modules/CalcSetup.lua:847`, `Classes/ModDB.lua:185`).
- Existing Maces and amulets supply complete end-to-end fixtures; capability admission can
  then include Lunar/Pearlescent source bases. Armour slots remain a coherent later slice:
  local BASE/INC consumption, multiplicative quality, local and global rounding, per-level
  bases and movement penalties require their own full source parity (`Classes/Item.lua:2521`).

Acceptance should include cold/warm original receiver branches at fractional, negative and
half-integer boundaries, exact parser/Global evidence, composed Spark/Mace passive/item
builds, export reimports, custom data, private bindings and allocation-free repeated calls.
This is existing roadmap scope, with no product-direction question outstanding.

### Assembly release measurements

The new [assembly benchmark](../examples/benchmark_assembly.rs) runs without the PoB
feature on the AMD Ryzen 9 9950X3D (16 physical / 32 logical cores). Its 1,806 legal Mace
selections cover 129 class/tree/attribute allocations, eight classes, all 23 named
ascendancies, three attribute options, six equipment selections and seven support loadouts.
There are 962 distinct full-metric checksums. All 24 samples and 16 separate complete
native document comparisons agree. Independent PoB parity remains a separate suite.

Median evaluations/second, three repeats per worker/mode:

| Workers | Fresh admission + measure | Already-admitted measure |
| --- | ---: | ---: |
| 1 | 68,396 | 6,699,848 |
| 2 | 75,503 | 8,994,672 |
| 4 | 113,551 | 18,529,430 |
| 32 | 458,867 | 121,230,974 |

Samples lasted 1.38–1.72 seconds, except the 32-worker reused-handle samples, whose
100-million-call cap shortened them to 0.811–0.826 seconds. Fresh admission includes
selection cloning, structural/data resolution, actor assembly, requirements and handle
allocation/destruction. Reused-handle measurement excludes actor admission and repeats
skill calculations from prepared actor resources. Both include Rayon and complete metric
checksums; neither measures XML/JSON, objective/proposal/archive costs or optimizer quality.
Calibration performs 13,624,464 attempts outside 460,673,835 timed sample evaluations.

Dataset/backend setup takes 227.56 ms, source catalog 4.39 ms, numeric components 0.48 ms
and admission of all 1,806 selections 30.70 ms. The catalog retains four item components,
1,142 passive programs, 185,280 bytes of compiled actor-owned storage and 3,516 source XML
bytes. The prepared evaluator estimates 1,550 component bytes, no XML and no candidate
results; scratch uses 536 inline bytes. These component estimates exclude shared data,
collection/allocator metadata and thread stacks, and are not peak process memory.

Exact measured executable SHA `e7788b326b0fd8592d8fd3fbbcc773346ca3f7bdd3bb2bcbab7fb0aae33168a5`;
benchmark source SHA `7f082b41d3c17a824d8aaa7f39d59c47100ebeccf49a6be5898d3fb4b3347f6a`;
raw JSON SHA `69ca88a52edb3544c1a0c0709b7f548a2265fa5ae8d63e1405b09151a35fe052`.
The measured binary precedes the later source-preparation error isolation fix; these
numbers belong to those exact fingerprints, rather than a subsequent rebuilt executable.
Full identities, min/max distributions and scope: `runs/assembly-benchmark-summary.{md,json}`
and `runs/assembly-benchmark-release.json`. Whole CLI reproduction follows separately.

### Complete release search reproduction

After all tests/builds finished, **24 complete native-only CLI searches** (typed/document
by 1/2/4/32 workers, three repeats) produced identical feasible/infeasible archives,
verification results, ledgers, exact XML and data companions. Each run used the checked-in
problem with 1,000 allowed calculations, 10,000 proposals and 32 rounds. Each stopped at the
round budget with **7,842 source admissions/proposals, 5,561 duplicates, 1,379 rejected states
and 904 full calculation attempts**: one baseline, 902 search evaluations and one fresh
finalist verification. There were no calculation failures or discarded late results.
The best verified diagnostic result has 148.6940653125 selected hit DPS and 115 maximum
Spirit against the configured floor of 110; no optimality or obtainable-affix claim follows.

Median command-reported elapsed milliseconds, including data/source/catalog preparation
and search, excluding final report publication:

| Workers | Typed | Full document |
| --- | ---: | ---: |
| 1 | 387.06 | 925.48 |
| 2 | 342.96 | 684.91 |
| 4 | 327.86 | 520.26 |
| 32 | 354.38 | 434.48 |

Process timings are separately recorded. Data initialization, proposals and archives limit
scaling on this small problem; kernel throughput must not be presented as whole-search
throughput. These are repeatability and integration measurements, not the planned 5–30
minute optimizer-quality experiments.

Final executable SHA `f389737393e7971156a58fb11929548b8f5c9f96f9d8796a0aa806cdc5a6268f`;
search implementation SHA `300701f311ab341890c7a57fe6b772c594cf0e7b7b875579d8995efd4d592c5c`;
verified XML SHA `6e67c48da964705bdb491fca212e75d9a165834b444d1463e7661905acfcf799`.
Raw reports, exports, companions and distributions are in `runs/assembly-release-search/`;
summary SHA `64d40a8323696b1a09bd6adfe01922af9b51029db688f2c2c09babff77e407e7`.
The developer reproduction script is `runs/verify-assembly-release.py`.

## Shared actor attributes and maximum resources - 2026-09-08

Starting point: `ffbca64`. The [actor design](actor-resources.md) specifies the data,
calculation, import and prepared-search seams. No change to the end-state native target
or source pin is implied. Full native replacement remains unfinished.

| Checkpoint | Current state |
| --- | --- |
| Injected data | Schema **6**, 116 literal actor rules, 40 source precision records, three actual Spirit quest closures/defaults and portable ordered actor modifier records. Package **198,104 bytes**, SHA-256 `7040fdf73dfd1500bb84b91c38944f1c4149524f30c962c5696a70115a4cc010`. Two fresh source extractions reproduce package/tree/evidence bytes; evidence SHA `5d54b6b0384e5a66302bc21fbca2ac92fd4bec6125e555ba88c3a5609d15ab30`. **36 data tests**, ten source-oracle tests (including 1,206 actor-parser comparisons), extraction/worker/CLI checks pass. |
| Shared kernel | Exactly two attribute passes, twelve comparison conditions, inherent bonuses, maximum Life/Mana/Spirit and global Accuracy share one Rust stage. Prepared components are dataset/input bound and retain no source database. **84 engine tests** pass, including **1,244 cold/warm actor source comparisons**, 384 empty/generic compatibility combinations, raw donor/CI guards and zero allocations across 5,000 prepared/compatibility calls. |
| Source/import | Block and legacy custom-modifier source are preserved. Configurable grammar and private attribute requirement preflight use the same engine. Narrow XML handling preserves actual legacy whitespace behavior. **79 import tests** pass. Decoded text/active lines/record and encoded-source limits bound evidence, with media-4 realization capped at 8 MiB. Review fixed missing Spirit defaults in native realization and bounded exact encoded actor source. |
| Native/CLI | Both profiles and typed Mace candidates consume prepared actors. Spirit metric, Spark/Mace evidence 2/4, problem 6/report 7 and actor search example are implemented. **39 native tests** pass, including all 4,410 states with 3,675 legal candidates under reviewed and injected actor data, exact typed/document equality and zero allocations across 8,000 mixed actor/local-weapon iterations. Four native CLI tests cover 84 states/70 legal/14 rejected/72 attempts across typed/document × one/four workers, empty attribute domains, strict scope and custom grammar/quest/precision replay. An additional explicit PoB CLI test validates three-attempt locked searches for blocks and legacy migration. |
| Full build parity | **60 fresh complete native/PoB build pairs and 13 exact native-export reimports pass**, covering Spark mapping/bossing, composed Monk/passives/rare Mace/two supports, numeric operations, inherent flags, zero overrides, two-pass feedback, LF/CRLF/tab legacy inputs, inactive unknown blocks, removal and Spirit quest selection. Mace's nested hand Accuracy is absent from the flat snapshot; actual-source Accuracy and complete-build HitChance/DPS provide the corresponding checks. Existing local-weapon, support, resistance, class/passive and independent golden suites also pass. |
| Integrated validation | The integrated workspace ran 502 tests: 501 passed and one old all-quests test omitted the newly exposed Spirit quests. The corrected seven-test Mace target passes, and one additional reference CLI test passes. **503 unique workspace tests are covered across the integrated run and focused checks**, with nine child helpers ignored directly and exercised by parents. **65 native-only CLI tests**, strict workspace/native-only Clippy, formatting, native runtime dependency isolation and five WASM libraries pass. No unresolved failure remains; this does not claim a second all-green full workspace command. Evidence: `runs/actor-test-coverage.json` and the named raw logs. |
| Release reproduction | **24 searches** (typed/document × 1/2/4/32 workers × three repeats) agree on archives, ledgers, exact finalist XML and dataset companions: **4,410 states / 3,675 legal / 735 LevelReq rejections / 3,677 attempts**. Fresh Spirit is 130. Custom grammar, Spirit quest values and precision produce 143 versus 142 at default precision; infeasible floors, attribute-empty domains and budgets 3/7 pass. Correcting the developer harness's initial precision expectation reused and hash-verified the completed timing artifacts, then ran only new edge checks. Final evidence: `runs/actor-release-edges/summary.json`, with original measured reports under `runs/actor-release-check/`. |
| Static preservation | All 1,082 pinned source-manifest files, source pin, tree/full snapshot, supplied originals and six independent golden pairs are unchanged. All twelve old data sections are identical; only the new actor section and expected manifest fields differ. Lockfile versions are unchanged: import adds the pure engine dependency, and engine adds host-only roxmltree for its existing shared XML oracle test. UTF-8, fences and local Markdown links pass. Evidence: `runs/actor-data-static-audit.json`. |
| Publication | Code **`4df9a4fcefe271011cd612fc1f8c7bc43d863c66`** is pushed to main. [Windows/Linux CI run 34199225766](https://github.com/Azaril/poe-optimizer/actions/runs/34199225766) **passes on Windows and Linux**. Updated exact-code snapshots: `runs/actor-main-ci-current.json`, `runs/actor-main-ci-jobs-current.json`. |

The raw resource kernel validates donor conversion and Chaos Inoculation function behavior;
full build profiles reject these incomplete downstream mechanics. Reservation, receiving
ES/armour/evasion, general normalized passive/item assembly, actor recursion and the supplied
minion build remain unfinished. Import adds a pure engine dependency for shared semantic
requirement checks; dependency versions and native-only isolation are verified.

### Actor release measurements

Windows x86-64, **AMD Ryzen 9 9950X3D, 16 physical / 32 logical cores**; project builds and
tests finished before measurement. The explicit actor benchmark set rotates all 3,675
legal candidates. Each invocation performs one baseline and 7,350 fresh equivalence
calculations before timing, with equal checksums across every chosen layer/worker/repeat.
No candidate results are cached.

Median evaluations/second, rounded. Complete-document and prepared-result layers use
20,000 calls × three repeats; inexpensive layers use 1,000,000 calls × five repeats.

| Workers | Full document | Prepared full result | Pure calculation | Timed typed snapshot | Typed + owned metrics |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1 | 4,485 | 20,221 | 15,164,980 | 5,634,508 | 4,315,967 |
| 2 | 4,975 | 22,951 | 18,247,143 | 7,574,891 | 5,584,290 |
| 4 | 8,491 | 34,072 | 36,834,583 | 15,178,998 | 11,272,788 |
| 32 | 28,621 | 129,506 | 240,286,421 | 103,558,262 | 73,879,798 |

Longer timed-snapshot samples span **5.58–5.82 million/s** at one worker and **95.6–105.8
million/s** at 32. The fastest samples still last only a few milliseconds; these are
bounded Mace API measurements, not sustained-load or general-build scaling evidence.
Raw distributions and summaries: `runs/actor-benchmark{,-fast}-isolated{,-summary}.json`.

In the first invocation, catalog preparation takes **37.80 ms**, actor/weapon/tree/support
component preparation **1.356 ms** and legal-handle construction **10.04 ms**. The numerical
prepared object accounts for **47,279 bytes**, with six weapons, 105 trees, 105 actors,
seven loadouts and one selector; no retained XML or result-cache entries. Handles use
117,600 inline bytes; comparison requests contain 13,448,890 XML bytes. These estimates
exclude shared data, catalog storage and allocator/Arc metadata; they are not peak RSS.
Actor preparation performs numerical work even though its full-build calculation count is zero.

Whole CLI runs include admission, preparation, scoring, fresh verification, serialization
and export. The example selects `wooden-physical`, Monk3/10364/24475 and Brutality I + Heavy
Swing at **113.35145523 DPS** and 130 maximum Spirit. Three-repeat wall times below are
minimum / median / maximum milliseconds.

| Workers | Typed candidates | Complete documents |
| --- | ---: | ---: |
| 1 | 247.4 / 248.5 / 384.0 | 1636.2 / 1644.8 / 1652.7 |
| 2 | 221.8 / 230.9 / 233.6 | 1068.3 / 1092.3 / 1096.7 |
| 4 | 234.4 / 240.2 / 250.1 | 726.9 / 727.2 / 735.8 |
| 32 | 244.5 / 255.7 / 267.5 | 510.1 / 510.4 / 513.8 |

One-worker median whole-search time is about **6.6× lower** with typed candidates. More
workers do not improve this small typed whole-search case; catalog/admission/report work
dominates. These results do not certify naturally obtainable rare items, realistic
mapping/bossing optimizer quality, complete builds or browser performance.

Artifact identities:

- Native-only release CLI: `de48ddce0331d4a70b631c9faeccf2e280937d8e004f9941980ba9f071e8be11`.
- Benchmark executable: `b3f89c413d1b870404f3fcc6685fd0e2741c00ffb5234a178ab837fbd80710e7`.
- Reviewed package: `7040fdf73dfd1500bb84b91c38944f1c4149524f30c962c5696a70115a4cc010`.
- Custom actor package: `e5304e9fdf138c3c94baeb77fdd7fed14518e4ecc251f6e48300df5d6f4e1862`.

### Next: normalized passive and equipment source assembly

Advance from authored custom modifiers to actual allocated-passive and equipped-item
sources feeding the shared actor stage. This is part of the existing native pipeline plan;
reservation is deferred until active-skill assembly supplies its prerequisites.

1. Extract complete ordered records for qualifying ordinary passive nodes and admit a whole
   node by supported mechanics. Preserve physical allocations, effective attribute choices,
   connectivity, class ownership, explicit point budgets and locks. The pinned tree contains
   329 nodes named Attribute/Strength/Dexterity/Intelligence; this is an inventory, not a
   promise that every node is supported. `PassiveSpec.lua` loads `AttributeOverride` and
   resolves selected options; do not treat attribute-node names as complete semantics.
2. Assemble equipment global records after exact local consumption. Reuse the current Maces
   first, then source-configured jewellery whose complete implicit/explicit behavior can be
   evaluated. `Item.lua` consumes local records before `CalcSetup.lua` merges item and passive
   sources. Spirit on a Spirit-bearing base can be locally consumed and scaled; forwarding
   every actor-looking line globally would double count it. Preserve order and provenance.
3. Replace fixed scalar-only passive assembly with normalized supported records. Compile
   source contributions once, then compose selected records using reusable indexed inputs
   and bounded worker scratch. Gear attributes introduce tree/equipment interactions, so
   actor-per-tree preparation is insufficient. Avoid a Cartesian actor-result cache and
   avoid rebuilding XML or an owned modifier database per candidate. Requirements and skill
   calculations must consume the same resolved actor result.
4. Validate complete Spark builds with connected attribute choices and actual attribute or
   resource jewellery, plus Mace builds combining local weapon rolls and global attribute,
   Accuracy and resource effects. Include a coordinated gear/tree change that flips an
   attribute condition and crosses a support requirement. Compare fresh PoB values and
   reimports, typed/document candidates, custom-data binding and serial/Rayon archives.
5. Reject disconnected paths, duplicate/conflicting overrides, incorrect slots, unknown
   local/global scope and any unsupported second effect. Preserve multi-dimension locks,
   zero-attempt empty domains, per-source evidence and source revision. Measure preparation,
   allocation and varied-candidate throughput after correctness passes.

This covers real ingredients in the supplied build without claiming its minion, granted
skill or defence mechanics. Reservation requires skill-local modifier queries, counts,
supports, forced reservations and condition feedback; receiving defensive resources also
require repeated stages. Keep those future boundaries explicit rather than admitting their
outputs from an isolated formula.

## Local weapon-modifier assembly - 2026-09-08

Starting point: clean `5bc00db`. This implements the prior source-audited weapon plan below.
No product-scope decision is blocking implementation; full native replacement remains
unfinished and the source pin/independent goldens are unchanged.

| Checkpoint | Current state |
| --- | --- |
| Injected data | Schema **5**, `poe2-native-profiles-v5`; five item-rule families carry exact textual templates, source numeric capture kinds, typed stat/operation mappings and local flags/keyword flags. Global `character.critical_chance_cap` is extracted from actual `CalcSetup`. Reviewed captures are unsigned integers; decimals do not silently enter through a broader parser. Old data sections/character fields and tree are preserved. |
| Source evidence | Two fresh extractor processes reproduce the reviewed package and evidence. Latest package **156,410 bytes**, SHA-256 `5a258250c10e8148c21193672f1af69ce0aeb16ce779081fca1882fc68b8006a`; evidence `3b12963f2aa1424943792b128fb45c408c7b744a429a7f8f5ba1a39d16d6ba5d`. Loader/source tests pass. Exact critical-cap BASE/source/flag/keyword/tag shape rejects fourteen mutations; two final fresh extractions reproduce identical package and evidence. Artifacts: `runs/local-weapon-source-cap-final-{1,2}/`, `runs/local-weapon-source-summary.json`. |
| Shared import | `parse_mace_item` and raw XML element admission retain rarity/name/base, exact source, quality/item level, explicit/effective equip level, ordered duplicate rolls and per-line provenance. Data owns grammar; unknown/global/conditional lines fail. Legacy `NormalMaceAlternative` aliases `MaceWeaponAlternative`. Native item evidence uses media version 3. Raw payload/CRLF fidelity changes catalog assembly and its fingerprint; PoB realization explicitly validates derived neutral `ModRange` children. **66 import tests and Clippy pass.** Catalog item definitions use the validated base rather than raw line positions, including rare names, leading whitespace and renamed custom bases. Canonical metadata headers retain their previous rejection behavior. |
| Native assembly | `CompiledGameData::prepare_mace_weapon` returns bound `PreparedWeaponStats`. Shared local consumption follows literal exact flags/keyword/first-tag semantics and preserves leftovers. Physical flat/INC and quality stages, elemental rounding/presence, local speed/critical rounding and offence cap order follow source. Typed preparation stores one weapon component per axis and defers preparation errors to selected candidates. Both Spark and Mace consume the shared source-derived critical cap and rounding path. **71 engine tests and Clippy pass.** |
| Typed and CLI integration | Native full documents and typed handles share item admission and prepared weapon calculation. New problem schema **5** / report **6** enables supplied modified/rare weapons with all existing class/tree/support axes. Schemas 1–4 reject the expanded item scope. **11 native candidate tests** pass, including both 4,410-state local-item matrices and zero allocations across 8,000 mixed modified-item calculations. **Five new CLI tests** pass: 84 states / 59 legal / 25 rejected / 61 attempts, typed/document × one/four workers, exact export replay, injected grammar/cap, zero-attempt LevelReq rejection and canonical header rejection under both schema 4 and 5. |
| Full source/build parity | Cold/warm original-source grids cover local consumption (40 queries over ordered 60-row inputs), raw assembly (644 cases), supplied-item/support pipelines (224), custom Mace bases/caps (24) and Spark critical rounding/caps (50). Fresh complete PoB comparison passes **48 cases + two baseline pairs + four reimports**, covering rare/support interactions, caps, speed/mitigation and modifier removal. Logs: `runs/local-weapon-build-parity.log` and the engine/workspace test logs. |
| Integrated validation | The first workspace command ran 457 tests: 456 passed, one native rejection contract failed, and nine child helpers were ignored directly but exercised by parent tests. Review restored canonical numeric headers and changed the now-supported local-physical rejection case to an unsupported global-to-Attacks line. Afterward **all 66 import, 38 native and 61 native-only CLI tests pass**, covering **460 unique workspace tests across the integrated run and focused reruns**, with no unresolved failures. This does not claim a second full workspace command. Final workspace/native-only Clippy, formatting, runtime-dependency isolation and five WASM libraries pass. Evidence: `runs/local-weapon-test-coverage.json` records raw logs and the focused agent tool transcripts separately. |
| Release reproduction | Full **4,410-state / 2,949-legal / 1,461-rejected** searches use **2,951 attempts** and agree at 1/2/4/32 workers in typed/document modes, three repeats each. Archives, fresh finalist XML and dataset companions match. Custom grammar, critical cap, signed resistance and support data preserve agreement for full infeasible runs and a three-attempt locked export. LevelReq-80 empty domains spend zero; partial budgets remain bounded. Evidence: `runs/local-weapon-release-check/summary.json`, `runs/verify-local-weapon-release.py`. |
| Static preservation | All 1,082 source-manifest file hashes/lengths verify; the PoB submodule is clean. Source pin, tree/full snapshot, supplied originals, six independent golden XML/reference pairs and dependency manifests/lockfile remain unchanged. Ten old data sections and every preexisting Character field are identical. UTF-8, fenced blocks and local links pass across 33 Markdown files. Evidence: `runs/local-weapon-static-final-audit.json`. |
| Publication | Code **`da168e94410730b8ee97764d54feb126484d12d3`** is pushed to main. [Windows/Linux run 34193804854](https://github.com/Azaril/poe-optimizer/actions/runs/34193804854) is **successful on both platforms** (confirmed at the actor checkpoint). Exact-code snapshots: `runs/local-weapon-main-ci.json`, `runs/local-weapon-main-ci-jobs.json`. This following publication update changes documentation only. |

Native-only calculations use no Lua or per-candidate subprocess. The PoB backend stays an
explicit parity/reference selection. The synthetic local-weapon example demonstrates
calculation/interaction contracts, not naturally obtainable affix sets or optimizer quality.

### Local-weapon release measurements

Windows x86-64, **AMD Ryzen 9 9950X3D, 16 physical / 32 logical cores**; tests and builds
completed before these measurements. The developer harness rotates 2,949 legal mixed
candidates and performs one baseline plus 5,898 explicit pre-timing equivalence calculations.
All selected modes, worker counts and repeats share the same checksum within an invocation.
The new `--candidate-set local-weapons` selects supplied weapon/support axes while retaining
the harness's fixed template, selected-data tree set and DPS query; example objectives,
locks and search settings are not benchmark inputs.

Median evaluations/second, rounded: complete-document/prepared-result modes use 20,000
calls × three repeats. The inexpensive modes use 1,000,000 calls × five repeats. Pure
calculation excludes metric/deadline work; each API layer performs different work.

| Workers | Full document | Prepared full result | Pure calculation | Timed typed snapshot | Typed + owned metrics |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1 | 16,881 | 38,089 | 16,364,844 | 5,726,944 | 4,374,325 |
| 2 | 18,993 | 45,803 | 19,048,889 | 7,873,185 | 5,717,033 |
| 4 | 26,722 | 62,378 | 38,983,011 | 15,969,874 | 11,347,943 |
| 32 | 108,841 | 255,397 | 251,130,085 | 107,109,959 | 75,163,669 |

The longer timed-snapshot samples span **5.40–5.87 million/s** at one worker and
**97.5–109.9 million/s** at 32. Some numerical samples still last only 4–14 ms; these are
bounded Mace API measurements, not sustained-load scaling or general-build throughput.
Raw distributions: `runs/local-weapon-benchmark{,-fast}-isolated.json`, with matching
`-summary.json` companions and `runs/summarize-local-weapon-benchmarks.py`.

In the first invocation, catalog preparation takes **36.06 ms**, numeric-axis preparation
**0.081 ms** and legal-handle construction **10.42 ms**. The prepared object accounts for
**17,831 bytes**, six weapons / 105 trees / seven loadouts / one selector, zero retained XML
and zero result-cache entries. Handles occupy **94,368 inline bytes**; comparison document
requests contain **9,956,826 XML bytes**. These estimates exclude shared data, catalog storage,
allocator metadata and Arc overhead; they are not process peak-memory measurements.

Whole CLI timing includes preparation, admission, numerical evaluation, scoring, verification,
serialization and export. The same diagnostic problem returns **99.07821 DPS**, selecting
`wooden-physical`, Monk3/10364/24475 and Brutality I + Rapid Attacks I. Three-repeat wall-time
distributions below are minimum / median / maximum milliseconds.

| Workers | Typed candidates | Complete documents |
| --- | ---: | ---: |
| 1 | 234.2 / 236.7 / 387.0 | 666.3 / 668.1 / 685.3 |
| 2 | 229.4 / 230.3 / 231.9 | 547.2 / 548.1 / 548.4 |
| 4 | 240.0 / 245.3 / 245.6 | 415.6 / 417.7 / 431.2 |
| 32 | 248.6 / 252.3 / 264.3 | 354.1 / 361.6 / 367.6 |

The one-worker median is about **2.8×** faster through typed candidates. More workers do
not improve this small typed whole-search case; catalog/admission/reporting dominate.
Numerical speed does not establish mapping/bossing search quality, naturally obtainable
rare items, browser performance or broad native parity.

Artifact SHA-256 identities:

- Native-only release CLI: `57e787af992532fe5ffc64d748e2d49f7b188080a133e077c6e0665b06d5ed88`.
- Benchmark executable: `6bb82fa1c8b31a10e9837e3277fc2e7d469d75627d5662acb7b98fe835e4d672`.
- Reviewed schema-5 package: `5a258250c10e8148c21193672f1af69ce0aeb16ce779081fca1882fc68b8006a`.
- Injected custom package: `01e45c2449e2249b6da2ea82c4375456cd6b58ef04efd54244b02cc56c5647d7`.

### Prior plan: shared actor attributes and maximum resources (implemented above)

Build a reusable actor-preparation stage consumed by both Spark and Mace. This advances the
existing complete-native-pipeline plan: their present attribute/Life/Mana/Spirit preparation
is duplicated, and `CharacterModifiers` exposes fixed scalar fields. The modifier query and
scaling primitives exist separately; this phase must connect them to actual actor inputs.
No product-scope answer is required and no source upgrade is implied.

1. Introduce data-owned normalized global modifier records with source/order, operation,
   flags, keyword flags and only audited tags. Build immutable compiled actor inputs and a
   `PreparedActorResources` value bound to the exact compiled dataset. Retain old scalar
   adapters only as compatibility boundaries; unsupported selected records must reject.
2. Route existing base/quest/class/tree attribute and maximum-resource effects through the
   shared stage first. Derive precision and patch-dependent rules from injected data;
   do not silently inherit the primitive modifier module's pinned precision defaults.
   Admit additional records by supported target/operation/tag semantics, with provenance,
   rather than adding another selected-node or item-name allowlist.
3. Translate the actual source order: `CalcSetup.lua` base records; `CalcPerform.lua` exactly
   two attribute passes and their comparison conditions; `CalcDefence.lua` maximum-resource
   function. Preserve zero-valued overrides, conversion/Extra/INC/MORE/Total ordering,
   rounding/minimums and Chaos Inoculation effects where the complete mechanic is supported.
   A generic fixed-point loop would change the source behavior.
4. Validate original-source cold/warm grids for BASE/INC/MORE/OVERRIDE zero, negative and
   rounding boundaries, conditions, query/layer/source ordering, attribute ties and second
   passes. Reject unknown flags/tags and invalid injected rules. Pair full fresh Spark and
   Mace builds, typed/document paths, requirements, locks and exported reimports.
5. Preserve direct Rayon sharing, allocation-free prepared numerical calls, selected-data
   binding, native-only dependency isolation and portable WASM library compilation. Measure
   preparation and runtime costs separately after correctness checks pass.

Maximum pools are the bounded first integration. Reservation, resource-to-ES/armour/evasion
receiver transfers, actor-target recursion and minion inheritance remain explicit gaps.
Do not admit conversion items by calculating only the donor side: PoB invokes resource
calculations repeatedly as those receiving defences are assembled. Broader active skills,
item sources and supporting actors must reuse this stage as coverage grows.

## Typed native candidate evaluation - 2026-09-08

Starting point: clean `f2985be`. This implements the previously planned typed-candidate
slice below without changing package, tree, source pin, independent goldens, candidate
identity inputs or report schema versions. Local validation and publication are complete;
exact-commit hosted Windows/Linux validation passes.

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
| Publication | Code **`f022d1925a3d21ef595f50eade3ac279793ec517`** is pushed to main. [Windows/Linux CI run 34190091048](https://github.com/Azaril/poe-optimizer/actions/runs/34190091048) **passes on both platforms**. Final exact-commit snapshots: `runs/typed-main-ci-current.json`, `runs/typed-main-ci-jobs-current.json`. This following publication update changes documentation only. |

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

### Planned local weapon-modifier slice (implemented above)

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
  use schemas 2–7 according to declared item/tree/support/actor scope; problem 6 uses report 7.
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
  declares ten player queries: Spark returns ten finite resource/resistance/hit values;
  Mace returns nine finite values plus explicitly unavailable `selected_average_hit` because
  the contract does not aggregate attack hands. [PoB](../crates/poe-optimizer-pob/src/metrics.rs)
  declares 16 definitions and 18 actor queries, including diagnostic EHP and five maximum-hit
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
  shared actor attributes/resources and conditions, nine ordinary-entrance modifier fields and five signed resistance fields.
  Source-executed numerical and XML compatibility tests cover these slices and the shared import boundary. Broader tags,
  general passive/equipment source assembly and full build pipelines remain explicit gaps. The numerical
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
| Native Rust calculation replacement | Active; connected admitted ordinary passives/attribute choices, all class/ascendancy identities, four resistance ascendancy passives and supplied weapon/amulet actor modifiers supported by restricted Spark/Mace pipelines; injected data and lazy native graph search implemented | Class/entrance materialization and explicit finite search rules are implemented; retain reviewed source compatibility, then broaden passive/modifier extraction, actor/skill coverage and full offence/defence while preserving differential parity and strict admission. Typed mixed-candidate preparation and API/whole-search measurements are implemented for the bounded Mace catalog; realistic broader performance, optimizer quality and browser execution still need evidence. |
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
      declare ten player queries including maximum Spirit, with Mace average hit explicitly unavailable. PoB coverage
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
      external loading, a bounded pinned source extractor and schema-6 data-bound controlled
      catalog/requirement rules, local item modifiers and bounded class/entrance/support composition. Broader source
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
| 2026-09-08 | `4845efe` | Added injected schema-4 support loadouts and all seven bounded Mace combinations. Local validation and Windows/Linux CI pass. Next: typed native candidate preparation. |
| 2026-09-08 | `f022d19` | Added private typed native candidate preparation, stack metrics and a fresh full-document verification hook. All 421 workspace and 56 native-only CLI tests, lint, portable checks and release comparisons pass. Identical bounded Mace search results take 164.5 ms versus 397.0 ms one-worker median; full native parity remains unfinished. Pushed to main; exact hosted run `34190091048` passes on Windows and Linux. Next: injected local weapon-modifier assembly. |
| 2026-09-08 | `da168e9` | Added injected schema-5 local weapon rules, normal/rare source-preserving admission, prepared numeric weapon assembly and problem/report schemas 5/6. Integrated validation plus focused review reruns cover 460 workspace tests, with 61 native-only CLI tests passing and no unresolved failures. Source/build parity, release reproduction, lint and portable checks pass. Typed/document searches agree across 1/2/4/32 workers; one-worker median 236.7 ms versus 668.1 ms for this diagnostic domain. Pushed to main; exact hosted run `34193804854` passes Windows/Linux (confirmed at the actor checkpoint). Next actor phase is implemented above. |
| 2026-09-08 | `4df9a4f` | Added schema-6 injected actor records, source configuration parsing, shared native attributes/resources and requirement checks, Spirit metric and problem/report schemas 6/7. Validation covers 503 workspace tests and 65 native-only CLI tests across integrated/focused runs, with no unresolved failures; source/build parity, zero-allocation prepared calls, release reproduction, Clippy and WASM checks pass. Full search agrees across 1/2/4/32 workers; one-worker typed median 248.5 ms versus 1,644.8 ms for this diagnostic domain. Pushed to main; exact CI run `34199225766` subsequently passed on Windows and Linux. The next normalized passive/equipment phase is recorded above. |
| 2026-09-08 | `e42760d` | Added schema-9 injected fixed Helmet/Gloves/Boots, source item formatting, shared native local components and player Armour/Evasion rating objectives; graph problem 9/report 10. 618 workspace + 82 native-only CLI tests pass across reconciled runs, with 33 fresh PoB pairs, 11 export reimports, source oracles, Clippy, WASM and preservation checks. All 24 release searches agree and the exact finalist matches PoB. Pushed to main; exact-code CI `34221294167` passed Linux and failed a Windows LF-only fixture edit, reproduced and repaired in `56dcbc5`. Body Armour/movement follows below. |
| 2026-09-08 | `56dcbc5` | Added schema-10 injected Body Armour and shared movement, source-generated penalties, case-normalized grammar, movement objective metrics and graph problem 10/report 11. 658 workspace + 87 native-only CLI tests pass, followed by 19 passing tests in complete affected targets after CRLF fixture repairs. Source/full-build parity, strict lint, WASM, preservation and 24 agreeing release searches pass; exact finalist matches PoB. Pushed to main; exact-code Windows/Linux CI `34228787060` has passed on both platforms. Next: verify hosted CI, then shared ActionSpeed and complete Spark/Mace timing including the source server-tick cap. |
| 2026-09-08 | `f1d8a1c` | Added schema-8 injected receiving data, shared source-ordered native Armour/Evasion/ES and resistance calculation, Lunar/Pearlescent bases, scenario/evidence bindings and graph problem 8/report 9. 580 workspace + 79 native-only CLI tests pass across integrated/focused runs; source/full-build parity, Clippy, WASM, preservation and release checks pass. All 24 complete searches agree; exact release finalist matches fresh PoB. Pushed to main; exact-code CI `34214745950` now passes on Windows and Linux. Local Helmet/Gloves/Boots components and rating metrics are implemented in the following checkpoint. |
| 2026-09-08 | `a2586a8` | Added schema-7 injected passive/equipment data, complete ordinary structure with 1,142 admitted source views, component actor programs, lazy typed Spark/Mace assembly and native graph search with exact locks/attribute choices. 556 workspace tests and 76 native-only CLI tests pass across integrated/focused runs; local Clippy, WASM, preservation and release reproduction pass. All 24 full searches agree across modes/workers. Pushed to main; exact CI run `34208805533` now passes on Windows and Linux. Shared receiving defences and resistance modifiers are implemented in the following checkpoint. |
| 2026-09-08 | `87bf005` | Added schema-12 injected configuration catalog, original-source metadata/default parity, caller-driven definition lookup and test-only isolation of the old fixture registry. Combined target coverage totals 740 passing Rust tests, plus 100 native-only CLI and 9 corpus tests. All five inputs preserve 228 scalar records and 110 fresh reference measurements; native broad-build support remains pending. |
| 2026-09-08 | `0da5c5c` | Added exact configuration source projection, caller-only CLI inspection, shared native/search scalar readers and schema-2 corpus evidence. 354 relevant workspace-target tests plus 95 native-only CLI tests and 9 runner tests pass; strict lint, WASM, isolation, preservation and docs checks pass. All five inputs project and repeat their PoB measurements, while native retains explicit root-layout exclusions. Ordered configuration definitions, container semantics and unresolved actor/action labels are audited; broader native admission and full parity remain unfinished. |
| 2026-09-08 | `b109b94` | Added caller-driven build inspection, shared strict MAIN admission and independent source/full-build metadata parity. All five originals retain exact bytes and now expose single-skill/set profile limits. B2 source inventory covers 15 sets/200 groups/541 occurrences; general model proposal awaits design discussion. Full native parity remains unfinished. |

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
