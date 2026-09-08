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
cargo run --locked -- search-experimental --backend native --problem examples/mace-search.json --jobs 4 --max-evaluations 10
```

A native-only executable excludes the PoB adapter, LuaJIT and native UTF-8 module:

```powershell
cargo build -p poe-optimizer-cli --release --no-default-features --locked
cargo run -p poe-optimizer-cli --no-default-features --locked -- evaluate tests/fixtures/calibration/spark-mapping.xml
cargo run -p poe-optimizer-cli --no-default-features --locked -- search-experimental --problem examples/mace-search.json --jobs 4 --max-evaluations 10
```

The native-only CLI defaults to native evaluation and controlled search. It also supports
import, metric discovery, objectives, saved-report reassessment and native benchmarking.
PoB worker, tree-extraction and calibration-harness commands are absent. The workspace still
contains optional reference/test packages; inspect the selected package and feature graph
when assessing deployment dependencies.

`poe-optimizer-import` owns bounded XML/share-code decoding, evaluation preflight and the
controlled Mace source projection. Its `controlled_mace` and `preflight` modules are pure
Rust and depend on shared core types. The old `poe_optimizer_pob::mutation` and
`poe_optimizer_pob::preflight` paths re-export them for compatibility. XML format
compatibility therefore adds no PoB runtime dependency to native search.

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

The lazy [`search-build` workflow](passive-equipment-assembly.md) uses `ControlledBuildCatalog`,
component actor programs and privately admitted `PreparedBuildCandidates` handles. It
combines equipment and passive fragments per candidate without a Cartesian catalog. The
remaining preparation details in this section describe the preserved legacy API.

Legacy controlled native search defaults to `--native-evaluation typed`. It uses the private
admission boundary in `poe_optimizer_import::controlled_mace`:

1. A fresh full-document baseline passes `bind_native_baseline` against the selected
   backend identity. `native_components` then exposes immutable source-derived weapon,
   tree and loadout axes, bound to that catalog, template, configuration and dataset.
2. `NativeBackend::prepare_controlled_mace` parses the template once through the same
   strict native parser and resolves the scenario and metric queries. It checks selected
   backend/data identity, then prepares numerical inputs for each weapon, a character
   input for each tree choice and compiled modifiers for each support loadout.
3. `validated_native_candidate` issues a private handle only for an exact registered
   candidate whose selected-data requirements pass. Class/ascendancy ownership and support
   eligibility were checked during catalog creation. The search caller separately applies
   its exact locks, connectivity and explicit point budgets before dispatch.
4. Candidate calculations read these prepared components by index and check an opaque
   catalog-instance binding in constant time. A handle from another catalog rejects even
   when its content fingerprints match. Arbitrary numeric inputs cannot manufacture a
   validated candidate handle.

`PreparedMaceCandidates` owns numerical axes, prepared metric selectors, a binding token
and shared compiled data. It retains no template/request XML, export, diagnostics or
candidate result cache, and stays usable after the source catalog/view is dropped.
Native component preparation and storage grow with the sum of axis sizes. Existing catalog
construction and eager alternative `xml_sha256` hashing still visit the Cartesian domain;
that separate cost is unchanged.

| Typed API | Result and responsibility |
| --- | --- |
| `PreparedMaceCandidates::calculate` | Fresh `MaceOutput`; no clock or OS services. |
| `PreparedMaceCandidates::measure` | Stack `NativeMetricSnapshot` containing the complete native Mace metric catalog and explicit availability. |
| `NativeBackend::evaluate_controlled_mace` | The same snapshot with elapsed time and native deadline/backend-identity checks. |
| `PreparedMaceCandidates::snapshot_measurements` | Requested metrics in native catalog order, converted into the shared owned `Vec<MetricMeasurement>`. |
| `PreparedMaceCandidates::footprint` | Axis/selector counts, deferred character/weapon-error counts and bounded component-storage accounting, excluding shared data/catalog and allocator metadata. |

The successful calculation, stack snapshot and timed snapshot calls allocate nothing in
the mixed-candidate allocation regression. Conversion to owned scheduler measurements
allocates the vector and query/reason strings. Search scheduling, archives and preparation
also have costs; the allocation result does not describe the whole search loop. Snapshots
always retain `diagnostic_only`, including when every selected metric is finite.

A valid custom package can contain individually admitted passive contributions whose
selected sum exceeds the numerical scope. Preparation retains such a character-axis error
and returns it only when that handle is evaluated. A locked-out combination therefore
cannot abort otherwise valid candidates, and selected failures retain the document path's
error kind/message and attempt accounting. This deferred behavior does not admit unknown
owners, unsupported operations or requirement failures.

Initial baseline and fresh finalist checks always use full document evaluation with exact
realization and export checks. `CandidateEvaluator::verify` performs that fresh finalist
calculation once in the shared search ledger before comparison/export. For differential
investigation, `--native-evaluation document` also evaluates every intermediate candidate
through the full document path. Both native modes use the local Rayon pool; PoB search
continues to use its explicitly selected supervised reference backend.

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

The [mixed-candidate benchmark](../examples/benchmark_mace_candidates.rs) separately
measures full document calls, prepared results, pure calculation, typed snapshots and owned
measurement conversion. It records axis/catalog setup and rotating candidate checksums;
fixed-input benchmark results above do not substitute for that comparison or whole-search
measurements. Current evidence and command results belong in the
[implementation record](implementation.md).

## Verification and expansion gates

The controlled Mace search accepts selected compatible datasets. Its schema-1 problem keeps
the original fixed Warrior profile; schema 2 adds all 31 admitted class/ascendancy identities
and zero or one class-local ordinary entrance, composed with the existing weapons/supports.
Problem schema 3 adds the four paid ascendancy choices with an explicit 0/1 ascendancy
budget; schema 4 adds the seven zero/one/two-support loadouts; schema 6 adds actor
configuration over schema 5 supplied local
weapon modifiers and rare payloads. Schemas 1–4 retain their original item scope. Point budgets come from the
caller and requirements use selected class attributes. This is still a finite restricted
catalog; new evaluator coverage is not automatically searchable.
The initial native baseline and fresh finalist verify the exact materialized source export,
class/root/skill projection, physical/effective passive IDs and configured effects, resolved
weapon/support evidence, fixed external configuration and backend identity. Intermediate
typed evaluations use the private prepared admission above. The top feasible candidate must
pass a fresh full-document evaluation with matching assessment before export. Derived
condition tables are allowed to reflect the candidate. Every result retains `diagnostic_only`; repeatability is not complete game-legality certification.

Parity uses unchanged independent full-build goldens, actual pinned Lua functions for
numerical boundary grids, and fresh PoB full-build comparisons for supported mutations and
exports. Adapter differential tests compare every legal finite candidate's complete Mace
output and metric values with the full native document path, including finite value bits,
custom data and explicit unavailable metrics. They also cover foreign bindings, invalid
owners/requirements, deferred numeric errors and clock failures. Agreement between the two
native adapters does not independently prove game mechanics or full PoB parity. Changing
source slices or implementation dependencies changes provenance; an identity label does
not authenticate an edited result file.

Expansion must preserve exact source/data identity, requested-versus-realized state,
unsupported-mechanic rejection and full-build parity. Broad tree/class/ascendancy, equipment,
skill/supporting-skill and defence/offence coverage is still required before general builds
can use the native evaluator. See [native calculation coverage](native-engine.md),
[controlled search](experimental-search.md), [tree projection](tree-projection.md) and the
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
