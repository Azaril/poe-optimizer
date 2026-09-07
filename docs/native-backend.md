# Native build backend and optional PoB reference

The native Rust backend evaluates complete supported build documents without a Path of
Building checkout, Lua state, DLL, executable or subprocess. PoB remains an optional
reference backend for parity checks and update investigation. The shared
`CalculationBackend` / `EvaluationEngine` interfaces let objectives, search and front ends
use either implementation without exposing Lua values or process APIs.

## Implemented build profiles

`native-poe2` currently supports two restricted document profiles:

| Profile | Character and skills | Equipment and encounter scope |
| --- | --- | --- |
| Spark | Unallocated Sorceress, no ascendancy, one level-1 quality-0 Spark | No equipment/supports; supported explicit normal, boss or Pinnacle encounter configuration |
| Mace Strike | Unallocated Warrior, no ascendancy, one level-1 quality-0 Mace Strike; optional level-1 quality-0 Brutality I | One normal Wooden Club or Smithing Hammer, quality 0–20, item level 1–100, no modifiers/implicits; normal enemies only |

Both profiles accept character levels 1–100, enemy levels 1–85, explicit supported encounter
inputs and supported quest/resistance settings. A native Mace request with a boss scenario
rejects. The optional PoB controlled profile also admits its supported boss scenarios;
selecting a backend does not imply equal mechanic coverage.

Spark returns nine finite metrics: life, mana, energy shield, four capped resistances,
selected average hit and selected hit DPS. Mace returns eight finite metrics and an explicit
`Unavailable` for `selected_average_hit`: the existing shared metric does not aggregate
per-hand attack averages. The resolved main-hand average remains diagnostic evidence. EHP,
maximum hits, Full DPS, minions and broader builds are outside native coverage. Unsupported
mechanics or unknown metric queries reject; there is no automatic Lua fallback.

`poe-optimizer-engine` owns numerical calculations and modifier semantics.
`poe-optimizer-native` projects source XML into immutable inputs, calls the selected pipeline
and builds the shared typed result. The formulas and constants come from pinned source;
unchanged independently generated full-build outputs are tests, not implementation data.
Default quest rewards and the resistance penalty are explicit in result context. Cached
source numerical outputs are ignored and removed from native exports. Encounter overrides
are written into exported configuration so re-import preserves the selected scenario.

## CLI and dependency separation

The default developer build includes the PoB reference feature and defaults to that backend.
Select native explicitly when using that build:

```powershell
cargo run --locked -- evaluate tests/fixtures/calibration/spark-mapping.xml --backend native
cargo run --locked -- evaluate tests/fixtures/calibration/mace-wooden.xml --backend native
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

`NativeBackend::prepare` returns an immutable `PreparedEvaluation`. Reuse avoids source
parsing/projection; every calculation still recomputes its supported numerical pipeline.
`PreparedEvaluation::calculate` has no clocks or OS calls. `evaluate_prepared` additionally
creates a fresh complete typed result, including context, measurements, validation, export
and diagnostics. Prepared inputs can be shared across Rayon tasks. Controlled native search
currently evaluates each materialized XML through the shared engine and uses its local
Rayon pool; PoB search uses separately supervised processes.

`EvaluationClock` is supplied by the host. Desktop `HostClock` uses monotonic time; browser
bindings must supply their own clock. Checks surround preparation, calculation and result
validation. Deadlines are cooperative rather than hard CPU preemption. The pure calculation
API leaves admission and cancellation to its host. Browser bindings and browser execution
tests remain separate work from successful WASM compilation.

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
`metric_checksum: null` because its average-hit metric is unavailable; eight finite values
remain visible in `sample_measurements`. The
`non_finite_or_unavailable_completed_results` counter counts results containing either
kind of missing finite value. Throughput is specific to these admitted profiles and does
not establish optimizer quality, browser speed or a speedup over PoB.

## Verification and expansion gates

Native controlled search checks the exact materialized source export, class/root/skill
projection, resolved weapon and support evidence, fixed external configuration and backend
identity. The top feasible candidate must pass a fresh evaluation with matching assessment
before export. Derived condition tables are allowed to reflect the candidate. Every result
retains `diagnostic_only`; repeatability is not complete game-legality certification.

Parity uses unchanged independent full-build goldens, actual pinned Lua functions for
numerical boundary grids, and fresh PoB full-build comparisons for supported mutations and
exports. Changing source slices or implementation dependencies changes provenance; an
identity label does not authenticate an edited result file.

Expansion must preserve exact source/data identity, requested-versus-realized state,
unsupported-mechanic rejection and full-build parity. Broad tree/class/ascendancy, equipment,
skill/supporting-skill and defence/offence coverage is still required before general builds
can use the native evaluator. See [native calculation coverage](native-engine.md),
[controlled search](experimental-search.md), [tree projection](tree-projection.md) and the
[living implementation record](implementation.md).
