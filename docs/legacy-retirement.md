# Legacy retirement inventory

Updated 2026-09-14 during the finite-catalog retirement checkpoint after `0a64ee3`. This is a living companion to
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
| `core/src/evaluation.rs`: BuildFormat, EvaluationRequest | Ordinary requests mandate PoB XML; options use source action/group indexes | D1 semantic build/scenario/query request; external document selector stays in import/oracle adapter. |
| `src/mutation_search.rs`: SearchExperimental, NativeEvaluation | Removed | Enum moved to its only consumer, build_search. Command/module and six obsolete problem examples deleted; no silent compatibility route. |
| `import/src/controlled_mace.rs`: ControlledMaceCatalog and related types | Removed | Implementation, exclusive tests and all callers deleted. Useful invariants moved to the retained candidate/import APIs. |
| `native/src/candidates.rs`: PreparedMaceCandidates | Removed | Shared NativeMetricSnapshot/Value extracted to metric_snapshot.rs; finite preparation/evaluation APIs and benchmark deleted. |
| `native/src/profile.rs`: NativeInput / Profile | Complete native preparation/calculation and build_candidates | Replace with a general resolved semantic plan; delete profile/XML parsing and dispatch. |
| `native/src/lib.rs`: NativeCalculation, PreparedEvaluation | Spark/Mace enum plus source/stage ownership and export | Shared typed result/plan over owned inputs; keep the same metric/coverage guarantees. |
| `native/src/build_candidates.rs`: PreparedBuildCandidates | Newer lazy candidate path still dispatches Spark/Mace | Migrate semantic candidate realization; generic search algorithm can remain. |
| `import/src/controlled_build*.rs`: TemplateProfile | Search-build template guards and XML rewriting | Replace production domain adapter; retain useful explicit locks, inventory and allocation contracts. |
| `engine/src/spark.rs`, `mace.rs`, `mace_supports.rs` | Closed pipelines also own shared reward/error/weapon types | Extract real numerical kernels and data-selected rewards/definitions, then delete profile pipelines. |
| `data/src/game_data.rs`, `engine/src/data.rs` | Mandatory spark/mace sections, two-weapon positional lookup and cached profile inputs | D2 independent semantic schema/compiled data; remove legacy fields with their last consumer. |
| `import/src/mace_item.rs` | Broader equipment also consumes rarity/payload parsing here | Extract general item envelope/rarity decoding before deleting the Mace-only parser. |
| `pob/src/mutation.rs` | Removed in prior D0 checkpoint | Its sole test target retired with the finite catalog in this checkpoint. No compatibility re-export remains. |

Source/typed-Lua program families and configuration UI/loader contracts require a separate
D1–D3 consumer migration. Move only tools that still serve offline acquisition or the oracle
into optional tooling. Do not preserve a generic source VM in every generated package under
a new name. Source-specific diagnostics remain optional evidence, not semantic identity.

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
