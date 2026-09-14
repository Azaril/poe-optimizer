# Legacy retirement inventory

Updated 2026-09-14 for the owned composition/binding/mapping checkpoint after `9aff988`. This is a living companion to
[architecture migration](architecture-migration.md); the [domain ADR](domain-architecture.md)
defines the target. None of the five originals yet completes native evaluation.

## What is actually coupled

Most Spark/Mace code is active obsolete architecture, not unreachable code. The native
backend and `search-build` still ultimately select those two profiles. The older
`search-experimental` entry point and its finite catalog have now been removed.
Removing only the skill files would break general actor/weapon callers and would not leave
a working general evaluator. The retirement unit is a dependency closure with its tests
and public entry points, not a filename pattern.

| Code/type | Current dependency | Retirement action |
| --- | --- | --- |
| `core/src/evaluation.rs`: BuildFormat, EvaluationRequest | Legacy numerical requests still mandate PoB XML; options use source action/group indexes | The owned_build request and codec now accept independent semantic inputs. Replace numerical callers after definition binding/general resolution lands; external selectors remain adapter-only. |
| `src/mutation_search.rs`: SearchExperimental, NativeEvaluation | Removed | Enum moved to its only consumer, build_search. Command/module and six obsolete problem examples deleted; no silent compatibility route. |
| `import/src/controlled_mace.rs`: ControlledMaceCatalog and related types | Removed | Implementation, exclusive tests and all callers deleted. Useful invariants moved to the retained candidate/import APIs. |
| `native/src/candidates.rs`: PreparedMaceCandidates | Removed | Shared NativeMetricSnapshot/Value extracted to metric_snapshot.rs; finite preparation/evaluation APIs and benchmark deleted. |
| `native/src/profile.rs`: NativeInput / Profile | Complete native preparation/calculation and build_candidates | Replace with a general resolved semantic plan; delete profile/XML parsing and dispatch. |
| `native/src/lib.rs`: NativeCalculation, PreparedEvaluation | Spark/Mace enum plus source/stage ownership and export | Shared typed result/plan over owned inputs; keep the same metric/coverage guarantees. |
| `native/src/build_candidates.rs`: PreparedBuildCandidates | Newer lazy candidate path still dispatches Spark/Mace | Migrate semantic candidate realization; generic search algorithm can remain. |
| `import/src/controlled_build*.rs`: TemplateProfile | Search-build template guards and XML rewriting | Replace production domain adapter; retain useful explicit locks, inventory and allocation contracts. |
| `engine/src/spark.rs`, `mace.rs`, `mace_supports.rs` | Closed pipelines also own shared reward/error/weapon types | Extract real numerical kernels and data-selected rewards/definitions, then delete profile pipelines. |
| `data/src/game_data.rs`, `engine/src/data.rs` | Legacy numerical consumers require Spark/Mace sections and positional/cached profile inputs | Owned_schema now loads an independent schema artifact with none of those sections. Offline effect conversion and native consumption are still needed before deleting legacy package fields. |
| `import/src/mace_item.rs` | Broader equipment also consumes rarity/payload parsing here | Extract general item envelope/rarity decoding before deleting the Mace-only parser. |
| `pob/src/mutation.rs` | Removed in prior D0 checkpoint | Its sole test target retired with the finite catalog in this checkpoint. No compatibility re-export remains. |

Source/typed-Lua program families and configuration UI/loader contracts require a separate
D1–D3 consumer migration. Move only tools that still serve offline acquisition or the oracle
into optional tooling. Do not preserve a generic source VM in every generated package under
a new name. Source-specific diagnostics remain optional evidence, not semantic identity.

## Owned input/schema/inventory checkpoint

The new `core::owned_build`, `owned_inventory`, `owned_definitions` and `owned_schema`
contracts have named portable consumers: structural document checking, inventory union and
availability binding, and Data's immutable owned schema package/index. `check-owned-input`
and `check-owned-schema` are thin host adapters. None requires legacy package sections,
source programs or UI callbacks. Item validation/canonicalization is shared instead of
adding a parallel implementation or constructing a fake character for inventory checks.

This checkpoint introduces the replacement input/data seams but does not yet replace a
legacy numerical consumer. No additional profile/kernel deletion is claimed. The remaining
retirement dependency is explicit: definition binding, five-case owned normalization and
general effect resolution must preserve the current useful numerical tests before legacy
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

## Finite-catalog retirement checkpoint

Removed the old `search-experimental` command, `controlled_mace` catalog,
`PreparedMaceCandidates` APIs, mixed-candidate benchmark and six exclusive problem examples.
The `NativeEvaluation` mode enum now belongs to build_search; shared stack metrics live in
native/metric_snapshot.rs. No replacement compatibility facade or hidden schema rerouting
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

This deletion does not remove the still-active closed evaluator or make search-build
general. The next dependency closure removes those paths alongside owned-model/data/plan
integration. There is no commitment to preserve experimental command schemas indefinitely.

## Remaining shared-type catches

- SparkQuestRewards is consumed by shared actor and import code; replace its six hardcoded
  quest fields with data-selected effects, not just a renamed struct.
- General weapon code uses MaceError/MaceWeapon/MaceWeaponData. Move to resolved definition
  inputs without changing tested local arithmetic or rounding.
- SourceFile and source lists currently bind general native/tree identity to both sample
  profiles. Runtime compatibility should bind to semantic package/operation versions.
- NativeMetricSnapshot still adapts the closed fourteen-metric profile output. Moving it
  out of the deleted finite API is extraction, not a general semantic result model.

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
