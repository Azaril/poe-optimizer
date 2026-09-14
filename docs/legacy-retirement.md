# Legacy retirement inventory

Updated 2026-09-14 against `1d55cea` and the D0 cleanup. This is a living companion to
[architecture migration](architecture-migration.md); the [domain ADR](domain-architecture.md)
defines the target. None of the five originals yet completes native evaluation.

## What is actually coupled

Most Spark/Mace code is active obsolete architecture, not unreachable code. The native
backend, `search-build` and `search-experimental` all ultimately select those two profiles.
Removing only the skill files would break general actor/weapon callers and would not leave
a working general evaluator. The retirement unit is a dependency closure with its tests
and public entry points, not a filename pattern.

| Code/type | Current dependency | Retirement action |
| --- | --- | --- |
| `core/src/evaluation.rs`: BuildFormat, EvaluationRequest | Ordinary requests mandate PoB XML; options use source action/group indexes | D1 semantic build/scenario/query request; external document selector stays in import/oracle adapter. |
| `src/mutation_search.rs`: SearchExperimental, NativeEvaluation | Old finite Mace catalog CLI; build_search imports the generic evaluation-mode enum | Extract the enum, remove command/module and obsolete schemas/examples. |
| `import/src/controlled_mace.rs`: ControlledMaceCatalog and related types | Old search CLI, native finite candidates, benchmark and tests | Delete after those callers retire; do not preserve a compatibility re-export. |
| `native/src/candidates.rs`: PreparedMaceCandidates | Finite catalog evaluator; also owns NativeMetricSnapshot/Value consumed by build_candidates | Extract shared measurement representation, then delete finite candidate APIs. |
| `native/src/profile.rs`: NativeInput / Profile | Complete native preparation/calculation and build_candidates | Replace with a general resolved semantic plan; delete profile/XML parsing and dispatch. |
| `native/src/lib.rs`: NativeCalculation, PreparedEvaluation | Spark/Mace enum plus source/stage ownership and export | Shared typed result/plan over owned inputs; keep the same metric/coverage guarantees. |
| `native/src/build_candidates.rs`: PreparedBuildCandidates | Newer lazy candidate path still dispatches Spark/Mace | Migrate semantic candidate realization; generic search algorithm can remain. |
| `import/src/controlled_build*.rs`: TemplateProfile | Search-build template guards and XML rewriting | Replace production domain adapter; retain useful explicit locks, inventory and allocation contracts. |
| `engine/src/spark.rs`, `mace.rs`, `mace_supports.rs` | Closed pipelines also own shared reward/error/weapon types | Extract real numerical kernels and data-selected rewards/definitions, then delete profile pipelines. |
| `data/src/game_data.rs`, `engine/src/data.rs` | Mandatory spark/mace sections, two-weapon positional lookup and cached profile inputs | D2 independent semantic schema/compiled data; remove legacy fields with their last consumer. |
| `import/src/mace_item.rs` | Broader equipment also consumes rarity/payload parsing here | Extract general item envelope/rarity decoding before deleting the Mace-only parser. |
| `pob/src/mutation.rs` | Only a re-export of import::controlled_mace, used by one PoB test | D0: removed; test imports the owning crate directly. Validation is recorded in implementation.md. |

Source/typed-Lua program families and configuration UI/loader contracts require a separate
D1–D3 consumer migration. Move only tools that still serve offline acquisition or the oracle
into optional tooling. Do not preserve a generic source VM in every generated package under
a new name. Source-specific diagnostics remain optional evidence, not semantic identity.

## Next bounded deletion: old finite-catalog workflow

1. Move NativeEvaluation out of mutation_search and the shared metric values out of
   native/candidates. Do not add replacement compatibility facades.
2. Delete search-experimental dispatch, controlled_mace, PreparedMaceCandidates and
   examples/benchmark_mace_candidates.rs with their exclusive callers.
3. Migrate unique generic validation and retire obsolete command/schema tests and Mace
   search examples. Update guides/help/README; do not silently route the old schema elsewhere.
4. Run affected search/native/CLI/import tests, strict lint and applicable native-only/WASM
   checks. Search for deleted symbols and remaining consumers before committing.

This deletion can proceed before D3; it does not remove the still-active closed evaluator
or make search-build general. The next dependency closure removes those paths alongside
owned-model/data/plan integration. There is no commitment to keep experimental command
schemas compatible indefinitely.

## Test and shared-type catches

- `tests/mutation_cli.rs` contains an unrelated extract-tree test: preserve it in a focused
  file before retiring that CLI test suite.
- Three `tests/build_search_cli.rs` comparisons call search-experimental and load its actor
  example. Replace that obsolete comparison dependency with direct, meaningful evidence.
- `native/tests/native_candidate_contract.rs` reads the old local-weapon problem. Preserve
  unique weapon vectors in fixture data before removing the finite-catalog test target.
- SparkQuestRewards is consumed by shared actor and import code; replace its six hardcoded
  quest fields with data-selected effects, not just a renamed struct.
- General weapon code uses MaceError/MaceWeapon/MaceWeaponData. Move to resolved definition
  inputs without changing tested local arithmetic or rounding.
- SourceFile and source lists currently bind general native/tree identity to both sample
  profiles. Runtime compatibility should bind to semantic package/operation versions.

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
