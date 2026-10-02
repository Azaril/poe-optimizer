# Legacy retirement inventory

Updated 2026-10-02 for the owner's aggressive retirement direction. This is a living companion to
[architecture migration](architecture-migration.md); the [domain ADR](domain-architecture.md)
defines the target. None of the five originals yet completes native evaluation.

## Retention rule

Retain a legacy path only for a named, currently useful consumer: original-PoB
reference evaluation, maintained data acquisition/import, or an independent
numerical regression that has not yet moved to the owned engine. An exported API,
experimental command, or test of abandoned implementation machinery is not by
itself a reason to retain that machinery. The owner explicitly authorized
aggressive removal on 2026-10-02.

Delete an obsolete path with its exclusive tests, helpers, fixtures and fingerprint
entries. Preserve useful game-mechanic evidence and move its assertions to the
retained boundary when needed. Git history is sufficient for abandoned experiments;
do not add compatibility wrappers or feature gates merely to keep them buildable.
Shared code with a live acquisition or numerical consumer needs a dependency audit
before removal. Record that consumer and the condition that ends its retention.

This rule also applies before all five builds work: replacing a valueless experiment
is not a prerequisite to deleting it. MVP work resumes at the next measured blocker
after each bounded retirement checkpoint.

## What is actually coupled

### NativeBackend library removal (2026-10-02)

Removed all forty tracked files in `crates/poe-optimizer-native`, sixteen exclusive
root integration targets and their private `parity_build` helper. Removed the
workspace member, root/PoB development dependencies and dedicated two-platform CI
job. Existing numerical report assessment still works without that library.

Three PoB consumers now retain their useful evidence without NativeBackend:

- Configuration observations retain all five original lifecycles, reused Build
  behavior, source hashes and structural controls; old native-prefix pairing is gone.
- Skill observations retain both JIT modes, five unchanged originals, processing
  order, selector precedence, scalar/map settings and source failure controls;
  native prepared-skill comparison helpers are gone.
- Item-set activation retains complete original Load receipts and its independent
  Import component comparisons, including mixed identity/node-write controls;
  the redundant public Native preparation comparison is gone.

The original PoB evaluator/calibration fixtures, all owned source witnesses and
Engine numerical source oracles remain. Removing old backend integration tests
does not establish equivalent owned full-build coverage. The five-build denominator
remains 0/5 until actual original requests evaluate and match.

Next: audit the source-shaped Import/Data/Engine dependency closure. Preserve named
offline acquisition and useful numerical references, separate the small shared
identity/formatting helpers used by owned conversion, and delete unused preparation
and closed-profile paths. The historical execution-inventory tool intentionally
still recognizes NativeBackend when inspecting older Git revisions.

### Legacy native CLI removal (2026-10-02)

Removed `prepare-build`, `search-build`, `benchmark-native` and the native branch
of `evaluate`/`metrics`, including their four private implementation modules,
seven exclusive CLI test targets and native export companion writer. The
assembly benchmark example, five obsolete search manifests and three exclusive
demonstration XML files are also removed. No replacement command or compatibility
alias routes those inputs into the owned evaluator.

`evaluate-owned` remains the native evaluation entry point. Optional `evaluate`
and `metrics` are explicitly PoB reference tools and reject the old `--backend`
and native `--data` arguments. The corpus intake runner retains caller-driven
PoB reference observations and source inspection, but rejects its obsolete
`native=CLI` lane. Saved evidence can still be assessed and compared.

At this earlier CLI checkpoint, NativeBackend and Rayon moved out of the normal
root dependencies and remained development dependencies for numerical tests. The
subsequent library removal above deletes NativeBackend entirely. This is not full
legacy isolation: Import still enables legacy Data/Engine
modules, and inspection/acquisition still uses the older source package. Shared
calibration fixtures, all five originals, the original PoB oracle, owned witnesses
and useful library numerical comparisons remain intact.

The subsequent library checkpoint above removed the old Native crate and its exclusive
integration tests. It retains independent source controls in the PoB configuration,
skill-preparation and item-set activation tests. Ordinary timing and resistance
already have lower-level pinned-source oracles; they do not require NativeBackend.
Only ordinary timing currently has an owned runtime consumer. Further Engine
cleanup must extract useful kernel checks from incidental Spark setup without
claiming equivalent owned full-build coverage.

### Class/UI experiment removal (2026-10-02)

Removed the PoB class-capture adapter, authenticated Common.new protocol, class
session observation API and exclusive configuration/control replay experiments.
The deleted integration targets are `profile_source_sessions`,
`source_configuration_callbacks`, `source_configuration_controls`,
`source_configuration_dispatch`, `source_configuration_level` and
`source_program_methods`. Their private Rust/Lua support files and class-observer
unit tests were removed with them. These tested an abandoned reconstruction of
PoB application objects; no production acquisition or owned evaluator called them.

The shared parser lowerer and generic observation machinery remain because
`source_program_parser_breadth` and `source_program_parser_scan` compare actual
parser inputs and results, including the five original builds. Their shared
helper is now `source_program_parser_observer.rs`; its unused class-capture half
was deleted. The retained generic observer rejects unsupported metatables and is
labeled reference tooling, not an evaluator construction model. Original-PoB
evaluation, owned source witnesses and the numerical calibration fixtures remain.

Six deleted implementation files were removed from the legacy extractor's
fingerprint inventory. A newly executed extraction correctly gets a new
implementation digest; the bundled historical game-data artifact is unchanged.
The follow-on audit also removed Data's source-class definitions and Engine's
class allocation, inheritance/proxy lookup and class-session bindings, plus their
exclusive synthetic tests. No maintained producer remained after the adapter's
deletion. Plain table reads/writes now live with the existing value heap; coverage
and fallback checks remain. The removed `ClassResolved` wire variant is rejected,
not silently interpreted as an ordinary table. Coverage bounds/local reference
checks and the non-class reserved-table append regression remain tested.

Data and Engine implementation fingerprints exclude the deleted class bodies.
At this checkpoint NativeBackend remained; the subsequent library removal above
retired it. The modifier parser, plain session/closure interpreter and useful
independent numerical comparisons remain live retirement dependencies. Neither
the new owned rule graph nor its data package depends on these class removals.

The exact deletion manifests are `runs/legacy-class-retirement-files.json` and
`runs/legacy-class-runtime-retirement-files.json` (33 complete files plus one
reduced/renamed helper; eight obsolete integration-test targets).
Validation results are recorded in the current implementation checkpoint.

Most remaining Spark/Mace code is reachable obsolete Import/Engine architecture.
The native CLI, NativeBackend crate and earlier `search-experimental` entry point
have been removed; retained Engine numerical tests still select those two profiles.
Removing only the skill files would break general actor/weapon callers and would not leave
a working general evaluator. The retirement unit is a dependency closure with its tests
and public entry points, not a filename pattern.

| Code/type | Current dependency | Retirement action |
| --- | --- | --- |
| `core/src/evaluation.rs`: BuildFormat, EvaluationRequest | Optional PoB reference requests use external documents and source action/group indexes | Retain for the named reference backend. The owned_build request, codec and schema binder accept independent semantic inputs; external selectors remain adapter-only. |
| `src/mutation_search.rs`: SearchExperimental, NativeEvaluation | Removed | Its later build_search consumer and remaining old search examples are now also deleted; no silent compatibility route. |
| `import/src/controlled_mace.rs`: ControlledMaceCatalog and related types | Removed | Implementation, exclusive tests and all callers deleted. Useful invariants moved to the retained candidate/import APIs. |
| Former `native` candidate/profile/preparation/result modules | Removed with the entire crate, including NativeMetricSnapshot/Value | No compatibility facade or replacement named-profile dispatch. Useful independent numerical and original-source evidence remains outside that crate. |
| `import/src/controlled_build*.rs`: TemplateProfile | Retained source-template admission/materialization and its tests; its search-build and NativeBackend consumers are removed | Audit the now-unused public preparation closure and retire it with exclusive tests. Preserve independently useful explicit lock, inventory and allocation contracts at retained boundaries. |
| `engine/src/spark.rs`, `mace.rs`, `mace_supports.rs` | Closed pipelines also own shared reward/error/weapon types | Extract real numerical kernels and data-selected rewards/definitions, then delete profile pipelines. |
| `data/src/game_data.rs`, `engine/src/data.rs` | Legacy numerical consumers require Spark/Mace sections and positional/cached profile inputs | Owned_schema now loads an independent schema artifact with none of those sections. Offline effect conversion and native consumption are still needed before deleting legacy package fields. |
| `import/src/mace_item.rs` | Broader equipment also consumes rarity/payload parsing here | Extract general item envelope/rarity decoding before deleting the Mace-only parser. |
| `pob/src/mutation.rs` | Removed in prior D0 checkpoint | Its sole test target retired with the finite catalog in this checkpoint. No compatibility re-export remains. |

Source/typed-Lua program families and configuration UI/loader contracts require a separate
D1–D3 consumer migration. Move only tools that still serve offline acquisition or the oracle
into optional tooling. Do not preserve a generic source VM in every generated package under
a new name. Source-specific diagnostics remain optional evidence, not semantic identity.

## Current deletion and remaining live dependency closures

Removed `import::item_slot_validity::is_item_valid_for_slot`, an unused compile-and-call
facade. Its only caller was a test helper, which now exercises the compiled
`SlotValidityProgram::new(...).check(...)` API already used by production activation.
All 13 semantic tests remain; no numerical golden or source fixture was removed.

Removed the unused `CompiledParserProgram::loop_states()` accessor after a repository-wide
caller search. The backing field remains in the live interpreter allocation/execution
path. This removes an unnecessary API; it does not retire the interpreter. The owned tree
compiler and normalizer add no call into that path. No additional Spark/Mace pipeline
removal is claimed by structural tree conversion.

The following observed calls prevent honest claims of complete retirement:

- Import's `controlled_build` and Engine's Spark/Mace pipelines still compile;
  their former NativeBackend preparation and candidate adapters are removed.
- General actor preparation consumes `SparkQuestRewards`; weapon preparation consumes
  `MaceError`, `MaceWeapon` and `MaceWeaponData`. Shared equipment decoding still uses
  item envelope types from `import::mace_item`.
- PoB source-program token/lowering code has an offline modifier-parser extraction
  consumer as well as paused probes. The source extraction identity also embeds those
  implementation files. Separate that acquisition closure before deleting its directory.
- The current CLI excludes PoB/Lua and NativeBackend by default but still enables legacy through Import.
  Isolated Data/Engine builds now exclude source/profile modules; the application consumer
  migration and owned-only shipped distribution remain explicit D5 gates.

The next retirement audits Import's source-template preparation closure and Engine's
remaining profile dependencies while preserving useful independent kernel/source
checks. It does not require implementing an equivalent obsolete profile in the owned engine. Real-build
work remains on the owned contracts. No new compatibility facade or named-skill
profile is allowed as a bridge. Public availability alone is not justification for
retaining an unused API.

## Compiled boundary and dead API checkpoint

The checkpoint sections below record their delivery-time scope. The current
dependency inventory above takes precedence where later retirements changed callers.

Data and Engine have an explicit transitional `legacy` feature. Isolated builds that
turn it off exclude the source interpreter, profile preparation, source-shaped package
and source-embedding fingerprint. Shared `timing::ordinary` and `resistance::ordinary`
remain source-independent; their unchanged legacy adapters moved to feature-gated files.
At that checkpoint the legacy Native fingerprint included both adapter and shared
numerical bodies; that crate and its fingerprint have since been removed.
Root CLI PoB support is opt-in. Full CI uses `--all-features` for reference validation,
and a separate compiler-input check prevents source code leaking into owned libraries.
Import still enables legacy in the current CLI; that dependency closure is not retired.
NativeBackend was subsequently removed entirely.

Removed seven repository-unused methods: `CompiledActorModifiers::bucket_count`,
`RuneBudget::{match_steps_used, output_bytes_used, callbacks_used, search_steps_used}`,
`ItemSetTransform::catalog_index` and `NativeItemLoadProvider::dependencies_mut`.
Their backing state and production callers remain. No numerical tests were deleted.

Removed public `parse_mace_item`, its private wrapper and its `None`/local-only branch.
Production already calls `parse_mace_equipment` with a physical item ID. The useful
metadata, range, grammar, ambiguity, source-attribution and XML decoding tests now call
that same entry point with an explicit test instance. Only the assertion preserving the
obsolete API's deliberately narrower grammar is removed. Shared equipment parsing and
its still-live profile consumers remain; this is a bounded deletion, not a general
item evaluator or the retirement of Spark/Mace calculation.

## Owned input/schema/inventory checkpoint

The new `core::owned_build`, `owned_inventory`, `owned_definitions` and `owned_schema`
contracts have named portable consumers: structural document checking, inventory union and
availability binding, and Data's immutable owned schema package/index. `check-owned-input`
and `check-owned-schema` are thin host adapters. None requires legacy package sections,
source programs or UI callbacks. Item validation/canonicalization is shared instead of
adding a parallel implementation or constructing a fake character for inventory checks.

This checkpoint introduces the replacement input/data seams but does not yet replace a
legacy numerical consumer. No additional profile/kernel deletion is claimed. The remaining
retirement dependency is explicit: complete selected five-case normalization and general
effect resolution must preserve the current useful numerical tests before legacy
request/profile/package consumers can be removed. New numerical work must use those owned
contracts; the legacy UI/source VM frontier remains paused.

## Composition/binding/mapping checkpoint

Owned projects now compose independently selected presets, and the binder consumes only an
injected schema index plus owned request. Offline registry/mapping lives in Import and reuses
Core typed addresses. Its external selectors never become Core runtime IDs. The thin
`bind-owned-input` CLI uses the same library boundary intended for GUI/web/search clients.
These are replacement prerequisites; they do not yet migrate a legacy numerical caller.
Removed the unused `ValidatedMaceWeapon::is_legacy_normal_payload` method and only its
classification-specific assertions. The tests still check exact header spelling/rejection,
quality values, modifier capture and equip-level behavior. No production caller used it.
The next retirement dependency remains all-five normalization plus shared effect resolution.
Do not delete unique numerical goldens or add another named-skill profile to bridge that gap.

## Historical finite-catalog retirement checkpoint

Removed the old `search-experimental` command, `controlled_mace` catalog,
`PreparedMaceCandidates` APIs, mixed-candidate benchmark and six exclusive problem examples.
At this checkpoint the `NativeEvaluation` mode enum moved to build_search, and shared
stack metrics moved to native/metric_snapshot.rs. Both later retired with the CLI
and NativeBackend crate. No replacement compatibility facade or hidden schema rerouting
was added. Compiler-discovered duplicate quest storage and unused tree/reference helpers
were removed with their last callers.

Tests exclusive to the removed enumeration/command/schema were retired. Shared laws were
migrated to the retained candidate API: ownership and detached lifetime, exact metric order
and unavailable values, injected identity/values, host deadlines, support costs, separate
point pools, receiver contribution and tamper rejection. The tree-extraction CLI test and
independent four-attack calibration tests now have focused targets. Five complete-build
parity matrices keep their numerical vectors and fresh PoB exports; local weapon inputs
are test fixture data. Custom escaped/Unicode definition identities, export roundtrip and
inconsistent-package rejection have focused CLI coverage.

Combined validation is recorded in [implementation](implementation.md); migration maps and
command receipts are under `runs/finite-catalog-retirement-01/`. These files record which
invariants moved and which implementation-specific tests were intentionally retired.

Migrated cases also exposed a source-order bug in the retained template importer: it now
validates a sorted copy of the support set while retaining authored order and rejecting
duplicates. Diagnostic materialization/evidence accepts an opaque prepared selection so
infeasible but computable reference builds remain testable. Search and typed candidate
workers still require admitted selections; no prepared-to-admitted conversion or legality
bypass is introduced. The prepared diagnostic methods retire with this legacy source adapter
when the owned semantic input path replaces its callers.

At that earlier checkpoint the closed evaluator and search-build remained. The
native CLI and library retirements above now remove those public workflows and
the NativeBackend crate. Import/Engine dependencies remain; experimental command
schemas are not preserved.

## Remaining shared-type catches

- SparkQuestRewards is consumed by shared actor and import code; replace its six hardcoded
  quest fields with data-selected effects, not just a renamed struct.
- General weapon code uses MaceError/MaceWeapon/MaceWeaponData. Move to resolved definition
  inputs without changing tested local arithmetic or rounding.
- SourceFile and source lists currently bind general native/tree identity to both sample
  profiles. Runtime compatibility should bind to semantic package/operation versions.
- NativeMetricSnapshot was removed with the NativeBackend crate; it is no longer a
  dependency or a proposed semantic result model.

## Validation that survives

Keep independent Spark bossing/mapping and four attack/weapon/support reference XML/JSON
fixtures, their PoB backend calibration tests and meaningful numeric vectors. Port native
profile goldens to shared semantic/kernel APIs; do not keep production profile wrappers
solely to compile old tests. Small named build fixtures used by general import, armour,
defence, timing and identity tests remain valid tests.

Retain generic laws: exact instance locks, separate point budgets, legality before scoring,
injected data identity, serial/Rayon consistency, fresh finalist budgets, provider invalidation,
source-independent semantic roundtrips and export no-overwrite. Old finite enumerations,
obsolete schema-rejection tests and duplicate harnesses can retire with the product API.

## Completion criteria

- No production skill-name dispatch or mandatory Spark/Mace package sections.
- Native input/plan/result contracts do not require PoB XML, control state or source VM data.
- Search edits semantic builds; XML export and optional PoB checks occur at adapter boundaries.
- Useful numerical and independent oracle coverage remains, and obsolete implementations,
  commands, schemas and support code are physically removed.

Detailed read-only audits are retained under `runs/architecture-reset-01/`; they are source
observations rather than delivered migration claims. Update this inventory when each
consumer moves or is deleted, including the validation scope.

## Owned normalization checkpoint

The new source collector, offline owned skill identity/role compiler, bounded lexical
precedence recipes and conservative normalizer have a named consumer: `normalize-owned`.
Only import/tooling modules see PoB source evidence and mappings. The normalizer accepts
owned artifacts and never uses `selected_view`, source evaluator stages, legacy game-data
loading or profile parsing. Its output is the same Core DraftSession used by direct authors.
No native numerical consumer is replaced at this checkpoint; no further Spark/Mace kernel
or golden deletion is justified yet. The finite catalog and unused payload classifier remain
retired. Full input correspondence, shared domain-effect resolution and numerical validation
are the next retirement dependencies. Do not resume the paused UI/source-VM frontier.


## Independent preset contributions

Owned project composition and draft finalization now select allocation-associated equipment
and configuration-owned rewards through typed memberships. Import normalizes Spec socket
uses into that ownership model; no selected-view replay or evaluator callback is added.
Version-1 owned project/draft persistence is rejected explicitly rather than maintained as a
second implicit path. No numerical profile consumer is removed by this structural change;
shared rule resolution and all-five selected semantic conversion remain the next dependency.

## Owned reward and reference routing checkpoint

Finite reward policies and the recorded projection ledger are Import adapters with named
CLI/breadth consumers. They do not extend the paused source VM or UI-construction path.
The offline quest metadata exporter uses existing structured configuration data and source
hashes; it adds no evaluator/interpreter. Reference reports live only in compressed tests.
No numerical profile is replaced by this checkpoint, so no additional Spark/Mace deletion
is claimed. The next deletion unit remains legacy numerical request/profile consumers
as shared effect resolution lands; the removed experimental catalog stays removed.

## Owned rule component checkpoint

Core's typed rule contracts, Data's independent rule package storage, Engine's bounded
compiler/executor and `check-owned-rules` have a named component consumer. They load no
legacy Spark/Mace sections or source VM programs. The [component boundary](owned-rules.md)
separates schema-bound storage, semantic compilation and explicit-fact execution from
actual build/provider resolution. Synthetic four-family tests and changed-data probes are
not original-build numerical parity; **0/5** remains.

No numerical profile caller was replaced by this slice, so no further profile/kernel or
golden deletion is claimed. The next numerical integration must convert real effects,
resolve them against owned requests, migrate a named legacy consumer with its useful
reference laws, and remove that replaced request/profile path and dependency closure in
the same checkpoint. Shared effect resolution must not become another permanent profile
or a source-program compatibility layer. Existing schema binding, durable ID allocation
and offline mapping are delivered prerequisites, not reasons to retain duplicate new APIs.

## Item/provider component checkpoint

`OwnedItemLinePolicy` has a production Import/CLI consumer and emits owned declarations;
its patterns and source annotations are not native rule operations. Engine's computed
skill-parameter projection similarly uses declared owned slots, without a generated PoB UI
group. New optional local-item parity tests depend on the actual pinned source and owned
rules, with no Spark/Mace profile helper. These establish component evidence needed for
migration. They do not yet replace an active whole-build numerical consumer, so no new
profile deletion is claimed. The next deletion remains coupled to actual semantic plan
integration, preserving the useful independent numerical vectors.


## Nullable item-level and unused helper retirement

Two unconsumed Import facades are removed alongside the owned model migration:

- `parse_mace_item_element` had only its own XML test caller. That test now calls the
  shared `decode_item_payload` followed by `parse_mace_item`, retaining exact CRLF,
  escaped/CDATA input and entity/comment/ModRange rejection. The shared decoder still
  serves equipment import; neither it nor the active item parser is deleted.
- `legacy_passive_actor_records` had no production caller. Its one-entrance wrapper test
  retires with it. Injected passive record values, exact allocation views and tamper
  rejection remain tested in Data's `passive_allocation` and Engine's `data_injection`.
  The then-active native tree consumer read resolved allocation actor modifiers directly;
  that consumer later retired with NativeBackend.

This removes unused compatibility code, not an active numerical profile. The smallest
next numerical replacement is local weapon preparation: Engine's `assemble_local_weapon`
and `CompiledGameData::prepare_mace_weapon`, including the closed Mace-keyed preparation
cache. The owned component plan must first bind exact equipment/modifier occurrences,
prove contribution closure and supply a real action consumer. Keep the generic
`consume_local_numeric` helper while armour uses it. Retire useful numerical laws into
that shared consumer in the same checkpoint; do not claim the complete profile/request
path retired when only this dependency is replaced.


## Source item attribution checkpoint

Removed the unused `ProgramOutput::source_output` accessor from
`engine/src/parser_program/runtime/mod.rs`. An exact repository symbol search found only
its definition; its backing source output, allocation/graph/work accessors and their tests
remain active and are retained. This deletes an unused facade, not the source interpreter
or a numerical profile. The source adapter now feeds the owned item converter without
production Lua/UI execution; optional original-source tests own the reference bootstrap.
The next numerical retirement still depends on general provider/action resolution.

## Timing migration audit and unused constant removal

The unused `engine::mace::CLASS_INTERNAL_ID` constant is removed. Repository-wide consumer
search found no use. This cleanup removes no numerical behavior or tests.

`import::actor_assembly::mace_action_timing` is still active through
`ControlledBuildCatalog::validate_prepared_native_realization`; both Engine timing wrappers
also serve full calculation. The replacement must migrate calculation and independent
realization validation together. Reusing an attached result as its own reconstruction
would discard the validation law. Keep the generic `timing::calculate` arithmetic and
reference vectors while moving its inputs to explicit owned action/equipment/actor routes.
Twister applies a skill attack-rate adjustment before reciprocal; a Sniper's innate rate
belongs to its minion actor. Neither belongs in another named-build wrapper.

A bounded native timing operation still needs a deliberate contract for independent finite
outputs when an intermediate or another output is nonfinite, plus the existing rounding
and signed-zero evidence. This is the next numerical design gate; the current owned
activation checkpoint does not retire an active profile. The detailed audit and test
inventory are in `runs/owned-activation-01/timing-migration.md`.

## Shared timing primitive checkpoint

The numerical body formerly inside `timing::calculate` now lives in the data-independent
`timing::ordinary` primitive. The old entry point maps fields and delegates, so legacy
reference cases and the new owned v4 timing expression execute the same arithmetic.
Independent channel classification preserves a finite capped rate/time even when another
channel is nonfinite. No PoB or `CompiledGameData` object enters the primitive.

This moves the numerical implementation out of the legacy data dependency; its preparation
callers remain active. `actor_assembly::mace_action_timing`, Spark/Mace evaluation and independent
realization validation still select/precompute their inputs through the legacy package.
Their replacement must bind actual equipment, supports, actor inputs and branch eligibility
through the owned plan before deleting those profile paths.


## Shared resistance and native metric checkpoint

Removed the duplicated resistance truncation/cap/floor bodies from `resistance::calculate`
and `actor_receiving::calculate`. Both call `resistance::ordinary`, whose explicit numeric
inputs and separate finishing step preserve signed values, override zero, multiplication
order and independent finite/nonfinite channels. The existing scalar adapter remains
BASE-only; the actor receiver remains BASE/INC-only. Their source admission has not widened.
Independent pinned-source tests continue to validate both callers.

`OwnedMetricPlan` and `evaluate-owned` consume the owned model and final stat
semantics. They do not call the old NativeBackend or parse a PoB document. The
Engine's `CompiledGameData` and its Spark/Mace dependencies, plus Import's remaining
source preparation, remain retirement work. The NativeBackend crate and public
CLI/search routes are removed. This earlier numerical checkpoint preserved active
numerical goldens.

Next paired deletion: general owned local weapon/action inputs must replace
`CompiledGameData::prepare_mace_weapon` / `assemble_local_weapon` and the corresponding
`ControlledBuildCatalog::validate_prepared_native_realization` preparation before removing
that profile dependency closure. `SparkQuestRewards` and shared weapon types still
have active callers; `NativeMetricSnapshot` is removed. The final distribution gate must remove
those source/interpreter/snapshot dependencies, not just the optional PoB/Lua crates.
