# Native build backend and optional PoB reference

The native Rust backend evaluates complete supported build documents without a Path of
Building checkout, Lua state, DLL, executable or subprocess. PoB remains an optional
reference backend for parity checks and update investigation. The shared
`CalculationBackend` / `EvaluationEngine` interfaces let objectives, search and front ends
use either implementation without exposing Lua values or process APIs.

Both profiles use [shared actor preparation](actor-resources.md) for attributes, inherent
bonuses, maximum Life/Mana/Spirit and global Accuracy. Data owns admitted modifier templates,
operations, condition tags, precision and Spirit quest settings. Native profile evidence is
Spark version 6 or Mace version 8. [Shared receiving defences](receiving-defences.md)
adds global ratings and resistance BASE/INC. [Local armour](local-armour.md) supplies
separately rounded equipment slot bases. [Body Armour and movement](body-armour-movement.md)
adds generated item penalties and shared movement output; conversion receivers, reservation and other
unsupported mechanics still reject.

## Implemented build profiles

`native-poe2` currently supports two restricted document profiles:

| Profile | Character and skills | Equipment and encounter scope |
| --- | --- | --- |
| Spark | One level-1 quality-0 Spark; supported class/tree selection described below | Optional supported amulet and fixed helmet/body-armour/gloves/boots; no supports; supported explicit normal, boss or Pinnacle encounter configuration |
| Mace Strike | One level-1 quality-0 Mace Strike; zero to two level-1 quality-0 reviewed supports; supported class/tree selection described below | One normal/rare supplied weapon from two selected base records (reviewed names: Wooden Club and Smithing Hammer), quality 0–20, item level 1–100, five reviewed local modifier families and explicit equip-level metadata; zero weapon implicits; optional supported amulet, fixed helmet/body-armour/gloves/boots and admitted global actor lines; normal enemies only |

Both profiles accept all eight pinned classes and 23 ascendancy identities, with implicit
roots and connected capability-admitted ordinary passives, including supported notables
and explicit physical attribute choices. They also
admit zero or one of four directly connected ascendancy small nodes: Warrior3/14960 (fire),
Druid2/61722 (elemental), Monk3/24475 (chaos) and Huntress3/17058 (negative elemental).
Owned typed effect values come from the injected package. Class
attributes affect resources and attack accuracy. Admitted entrance effects include skill
speed, skill-type damage increases and flat armour, evasion or energy shield. Shared physical
roots and class-specific node substitutions retain their distinct source identities.
Excluded passive views, other ascendancy passives, jewels, grants and
weapon-set allocations remain unsupported. Selecting an ascendancy identity does not imply
coverage for its other effects. The evaluator does not infer an available point budget
or certify equipment/gem requirements; search must separately apply explicit finite rules.

Both profiles accept character levels 1–100, enemy levels 1–85, explicit supported encounter
inputs and supported quest/resistance settings. A native Mace request with a boss scenario
rejects. The optional PoB controlled profile also admits its supported boss scenarios;
selecting a backend does not imply equal mechanic coverage.

Spark returns thirteen finite metrics: life, mana, Spirit, energy shield, four capped
resistances, selected average hit, selected hit DPS, Armour, Evasion and movement percentage.
Mace returns twelve finite metrics and an explicit
`Unavailable` for `selected_average_hit`: the existing shared metric does not aggregate
per-hand attack averages. The resolved main-hand average remains diagnostic evidence. EHP,
maximum hits, Full DPS, minions and broader builds are outside native coverage. Unsupported
mechanics or unknown metric queries reject; there is no automatic Lua fallback.

`poe-optimizer-data` owns the portable tree model, finite projections and an authenticated
bundled ordinary structure and explicitly admitted/excluded effective source views. Native builds load this data without
a PoB checkout. The bundle identifies retained and excluded records explicitly; it is not
a complete game database. Optional PoB extraction produces fresh artifacts for updates and
parity. See [tree projection](tree-projection.md).

Native results include a `native-tree` diagnostic attachment with the selected class and
ascendancy, physical allocations, effective passive stat lines and the bundled-data digest.
Use CLI `--raw` to retain these diagnostic attachments. The attachment uses schema/media version **3** with explicit attribute choices, source-view keys and separate ordinary/ascendancy used counts
and a kind on each paid physical/effective view. The tree evidence kind is
`native_source_resolution`; `point_budget_verified` is false. The
PoB-specific live passive observation field remains absent on native results, because native
source resolution cannot establish Lua object-reference observations. Player `armour` and
`evasion` are final ratings exposed as definition-schema-1 `rating_points` metrics and in
the native profile diagnostic. They do not represent mitigation or chance to evade.
`movement_speed_pct` uses schema-1 `percent`, with 100 representing baseline effective
movement speed. [Shared action timing](action-timing.md) adds `action_speed_pct`, with 100
representing baseline player action speed, and resolves supported action-speed sources for
both movement and offence. Party/linked-skill and other unrepresented producers still reject.

`poe-optimizer-engine` owns numerical calculations and modifier semantics.
`poe-optimizer-native` projects source XML into immutable inputs, calls the selected pipeline
and builds the shared typed result. Rust formulas and reviewed package values come from pinned source;
unchanged independently generated full-build outputs are tests, not implementation data.
Before resolving a native document, the bundle's rules revision, tree version and source
tree digest must agree with both numerical pipelines. A mismatch returns a backend contract
error, so data refreshes cannot silently pair a new tree with older calculations.
Default quest rewards and the resistance penalty are explicit in result context. Cached
source numerical outputs are ignored and removed from native exports. Encounter overrides
are written into exported configuration so re-import preserves the selected scenario.

## Injected game data

Native backend instances now own shared `CompiledGameData`; prepared evaluations retain
the same data and semantic identity. The loader accepts embedded or external package bytes
through one bounded validation path. Skill/item records, monster tables, rewards, balance
parameters and typed entrance effects come from configuration. Different datasets can run
concurrently, and incompatible prepared reuse rejects before calculation.

See [native data packages](native-data.md) for `--data`, trust, the package authoring helper,
identity/report schemas and export metadata. The strict structural-tree guard remains;
controlled native search consumes the selected snapshot too. Broader source/tree updates
and general mutation coverage still require further work.

## CLI and dependency separation

The default developer build includes the PoB reference feature and defaults to that backend.
Select native explicitly when using that build:

```powershell
cargo run --locked -- evaluate tests/fixtures/calibration/spark-mapping.xml --backend native
cargo run --locked -- evaluate tests/fixtures/calibration/mace-wooden.xml --backend native
cargo run --locked -- evaluate examples/native-witch-entrance.xml --backend native --raw
cargo run --locked -- metrics --backend native
```

A native-only executable excludes the PoB adapter, LuaJIT and native UTF-8 module:

```powershell
cargo build -p poe-optimizer-cli --release --no-default-features --locked
cargo run -p poe-optimizer-cli --no-default-features --locked -- evaluate tests/fixtures/calibration/spark-mapping.xml
```

The native-only CLI defaults to native evaluation and retains the restricted `search-build` workflow. It also supports
import, metric discovery, objectives, saved-report reassessment and native benchmarking.
PoB worker, tree-extraction and calibration-harness commands are absent. The workspace still
contains optional reference/test packages; inspect the selected package and feature graph
when assessing deployment dependencies.

`poe-optimizer-import` owns XML/share decoding, source adapters and preflight.
The finite controlled_mace API and redundant PoB mutation facade are retired. Remaining
source-shaped profile consumers are listed in [legacy retirement](legacy-retirement.md).

Evaluation also checks compatibility with the pinned PoB XML reader. XML syntax that it
silently reinterprets (such as numeric character references in attributes or whitespace
around attribute `=`, fragmented CDATA or a leading UTF-8 BOM) rejects before either backend calculates. Generic import still
preserves source bytes. Native additionally rejects literal attribute whitespace that XML
would normalize; the PoB reference keeps its supported multiline configuration values.
Native export applies edits with one bounded source-span copy,
including removal of large cached-stat sections.

## Preparation, parallel execution and clocks

`NativeBackend::prepare` returns an immutable `PreparedEvaluation` bound to the instance's
injected dataset. Reuse avoids source parsing/projection; every calculation still
recomputes its supported numerical pipeline. This document-oriented object retains its
request and export XML. `PreparedEvaluation::calculate` has no clocks or OS calls;
`evaluate_prepared` also creates a fresh complete result with context, measurements,
validation, export and diagnostics.

The retained [`search-build` workflow](passive-equipment-assembly.md) uses
`ControlledBuildCatalog`, `ControlledBuildDomain` and private `AdmittedBuildSelection`
handles. It combines equipment and passive fragments without constructing a Cartesian
catalog. Its numerical dispatch still selects the restricted Spark or Mace profile;
the graph-shaped search API does not establish general native build coverage.

1. `NativeBackend::prepare_controlled_build` validates the source template and fixed
   scenario once, requires the catalog's exact compiled-data instance, and prepares weapon
   components, support admission results and metric selectors. Introduced supports pass
   the same authored-loader checks during setup.
2. `ControlledBuildDomain::admit` resolves a selection with caller-owned `ActorScratch`,
   checks structure and selected-data requirements, and returns a private handle containing
   the prepared actor and selection. Domain rules retain explicit budgets and locks.
3. `PreparedBuildCandidates` checks the handle's catalog binding before calculation;
   timed execution additionally checks backend identity and exact data-instance ownership.
   Equal fingerprints cannot authorize a foreign handle.

The prepared evaluator retains shared compiled data, profile input, weapon/support
components, metric selectors and a private binding. It retains no XML or candidate result
cache. Actor preparation belongs to admission, not an eagerly enumerated tree axis.
Selected support, weapon or scalar failures remain candidate-local; this does not bypass
unknown-owner, unsupported-operation or requirement checks.

| Retained API | Result and responsibility |
| --- | --- |
| `PreparedBuildCandidates::calculate` | Fresh `NativeCalculation::Spark` or `NativeCalculation::Mace`; no host clock. |
| `PreparedBuildCandidates::measure` | Stack `NativeMetricSnapshot` with explicit availability. |
| `NativeBackend::evaluate_controlled_build` | Snapshot with host-clock deadline and backend/data-instance checks. |
| `PreparedBuildCandidates::snapshot_measurements` | Selected metrics in catalog order as owned `Vec<MetricMeasurement>`. |
| `PreparedBuildCandidates::footprint` | Weapon/support/selector counts, deferred support/weapon errors and bounded component-storage accounting; shared data and admitted handles are separate. |

The retained [candidate regressions](../crates/poe-optimizer-native/tests/native_build_candidate_contract.rs)
cover allocation-free successful calculation/snapshot calls for their admitted cases.
Owned metric conversion, admission, preparation, scheduling and reporting still allocate.
Snapshots retain `diagnostic_only`, including when all requested metrics are finite.

Search records a full-document baseline attempt when an initial candidate is admitted.
A failed imported baseline remains visible and does not suppress valid repaired alternatives.
`CandidateEvaluator::verify` performs a fresh, counted full-document finalist calculation
and exact realization/export checks. `--native-evaluation document` uses that document path
for intermediate candidates too. Both modes use the local Rayon pool. The retired finite
Mace API and its benchmark are not alternative executable paths.

`EvaluationClock` is supplied by the host. Desktop `HostClock` uses monotonic time; browser
bindings must supply their own clock. Checks surround preparation, calculation and result
validation for full document calls. Typed calls check the clock before and after numerical
calculation; preparation and owned scheduler conversion remain within the search host's
outer deadline. Deadlines are cooperative rather than hard CPU preemption. The pure
calculation API leaves admission and cancellation to its host. Browser bindings and browser
execution tests remain separate work from successful WASM compilation.

## Fixed-input throughput benchmark

Run release benchmarks with the same profile, mode, evaluation count and machine when
comparing worker counts:

```powershell
cargo run --release --no-default-features --locked -- benchmark-native tests/fixtures/calibration/spark-mapping.xml --mode prepared --jobs 1 --evaluations 10000 --timeout-seconds 30
cargo run --release --no-default-features --locked -- benchmark-native tests/fixtures/calibration/spark-mapping.xml --mode prepared --jobs 4 --evaluations 10000 --timeout-seconds 30
cargo run --release --no-default-features --locked -- benchmark-native tests/fixtures/calibration/spark-mapping.xml --mode document --jobs 4 --evaluations 10000 --timeout-seconds 30
```

Prepared mode validates the fixed source once and calls `evaluate_prepared` each iteration.
Document mode also repeats complete XML parsing and the shared engine contract on every
iteration. Both measure full typed API calls, including result construction, validation,
XML export and diagnostics. Timing also includes Rayon scheduling, ledger updates and
checksum observation. It is not a raw numerical-kernel measurement. No result cache or
uncounted calculation warmup is used.

The local pool accepts 1–64 jobs and 1–1,000,000 requested evaluations. A shared deadline
covers import, preparation and iterations; late results are discarded and workers are
joined. Reports separate preparation and iteration timing, count attempts/completions/
failures/late results, retain full backend identity and identify partial runs. Timers stop
at worker join, excluding report aggregation, serialization and output. Optional `--output`
requires a new path and rejects collisions before work.

A checksum is emitted only when every completed result contains finite values for the
entire requested catalog. It hashes sorted metric query/unit/schema/value bits and uses an
order-independent aggregate across workers. Spark satisfies that gate. Mace currently has
`metric_checksum: null` because its average-hit metric is unavailable; nine finite values
remain visible in `sample_measurements`. The
`non_finite_or_unavailable_completed_results` counter counts results containing either
kind of missing finite value. Throughput is specific to these admitted profiles and does
not establish optimizer quality, browser speed or a speedup over PoB.

The former finite Mace mixed-candidate harness is retired. Its layer-separated timings
remain historical evidence in the [performance inventory](execution-model-performance-inventory.md);
they cannot be replayed with a deleted example or transferred to the retained backend.
The current [assembly benchmark](../examples/benchmark_assembly.rs) measures fresh admission
versus already-admitted measurement for restricted diagnostic selections. It is a different
workload from fixed-input result construction and whole-search execution.

## Verification and expansion gates

The retired finite Mace search used problem schemas 1-6 to add class/ascendancy, support,
weapon and actor axes. Those schemas and their Cartesian catalog are historical, not
supported `search-build` inputs. The retained graph-domain workflow and its current problem
schema are documented in [passive/equipment assembly](passive-equipment-assembly.md).

Retained tests cover document-versus-typed calculation, injected data, selected metric
availability, foreign ownership, deferred failures and clocks. Independent numerical
oracles and full-build comparisons are still necessary: agreement between two native
adapters does not independently establish game semantics. Historical finite-catalog
comparisons remain evidence for their recorded profiles/data, not universal coverage.

New coverage follows [domain architecture](domain-architecture.md) and the
[owned-model migration](architecture-migration.md). Preserve meaningful numerical and
observable game-behavior tests when retiring legacy adapters. None of the five supplied
originals completes native evaluation. Current capability and validation remain in the
[living implementation record](implementation.md).

The Mace profile now uses [data-derived support loadouts](support-loadouts.md), with
prepared modifier aggregates and exact configured-support evidence (profile attachment
version 8, including exact item provenance, global actor records and prepared local weapon stats). Spark support coverage remains empty.

The [local weapon pipeline](local-weapons.md) parses supplied item text once, compiles its
injected rule mappings into `PreparedWeaponStats`, and uses that bound object in both full
document and typed candidate calculation. Exact payload/line provenance and local stats
are separate diagnostic evidence. Explicit `LevelReq` follows PoB's non-unique override
rule; item level does not supply equip level. Broader affix, acquisition and equipment
legality remain outside this calculation profile.
