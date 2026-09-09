# poe-optimizer

An experimental Rust project for constraint-driven Path of Exile 2 build optimization,
with potential Path of Exile 1 support later. The intended workflow is to import a
build, choose which build decisions may change, set encounter assumptions, and configure goals,
then search for good legal alternatives using a native Rust evaluator, with Path of Building
available as an optional parity reference.

Goals are user-configurable. Damage, resistance, EHP, and skill-selection examples are
illustrative; users choose objectives, constraints, and preferences from an extensible
metric catalog. The first usable optimizer must jointly search classes, ascendancies,
passives, equipment, support gems, and supporting active skills. Users can require 1..N
skills and retain 1..N exact equipped item instances, with independent locks on other
choices. Finite candidate pools bound each run without removing these search dimensions.
Candidate-derived skill effects are recalculated while external encounter assumptions stay fixed.

The initial quality target is configurable 5–30 minute runs, benchmarked in both bossing
and mapping contexts. Search must handle coordinated changes that escape local optima;
it returns verified best-found alternatives without promising a global optimum.

The planned engine uses reusable Rust core libraries and direct Rayon evaluation under
shared CPU/memory limits. Optional isolated Rust workers host the PoB reference through
`mlua` with the LuaJIT backend. The CLI comes first, with
structured run data and offline HTML reports; a later GUI can use the same APIs,
with Tauri as a candidate.

**Status:** experimental import and evaluation CLI with backend-neutral calculation and
evaluation APIs. It accepts PoB share codes/XML, explicit skill/action and encounter options,
and returns typed metrics with units, availability and structured coverage. Configurable scalar
objectives can be assessed during evaluation or against saved results without recalculation.
Each PoB request
uses a fresh `mlua` worker process with a deadline. Independent Spark mapping/bossing
references plus a four-case attack/weapon/support matrix validate host and metric extraction.
A parallel native Rust crate has parity-tested numerical helpers and numeric/conditional
modifier aggregation, data-driven condition producers and numeric scaling programs. A native build backend now parses
restricted Spark and Mace Strike profiles across all pinned class/ascendancy identities,
with connected capability-admitted ordinary passives, explicit attribute choices,
and selected ascendancy resistance passives,
and computes their supported resource,
resistance and hit metrics entirely in Rust. `evaluate --backend native` uses the same result/objective APIs, and a
native-only CLI build excludes Lua and PoB. General native build coverage remains in progress;
see [native backend and optional reference mode](docs/native-backend.md).
Canonical candidates and locks represent all six dimensions. A generic search kernel has
bounded parallel evaluation, feasible/infeasible beams, deduplication and fresh finalist
checks. The native [`search-build` workflow](docs/passive-equipment-assembly.md) lazily
searches connected passives, physical attribute choices, supplied weapons/amulets and fixed helmets/body armour/gloves/boots,
class/ascendancy identities and the admitted support loadouts. Compiled actor components
are combined per candidate; no Cartesian build or result table is constructed.
Legacy `search-experimental --backend native` searches supplied Mace item/support
choices and optional class/ascendancy/ordinary and ascendancy passive selections directly on Rayon with exact locks
and source-preserving mutations. Mace support choices include all seven zero/one/two-gem
loadouts from Brutality I, Heavy Swing and Rapid Attacks I, with values and eligibility in
[the injected data package](docs/support-loadouts.md). [Local item rules](docs/local-weapons.md)
also admit supplied normal/rare Maces with physical/fire damage, local speed and critical
modifiers. [Shared actor preparation](docs/actor-resources.md) adds configurable attribute and
maximum-resource modifiers with the same calculations for skill output and item/support
requirements; `player.spirit` exposes maximum Spirit before reservation.
[Shared receiving defences](docs/receiving-defences.md) adds source-ordered Armour, Evasion,
Energy Shield and resistance BASE/INC from configuration, gear and passives. Graph problem
8 exposes this scope with configurable defence constraints and report 9.
[Local armour equipment](docs/local-armour.md) adds separate item quality/local rounding,
288 injected fixed bases and player armour/evasion rating metrics; graph problem 9/report
10 searches this equipment scope. [Body Armour and movement](docs/body-armour-movement.md)
adds 114 body bases, source movement penalties and `player.movement_speed_pct` (100 is
baseline); graph problem 10/report 11 searches that scope. [Shared action timing](docs/action-timing.md)
adds actor action speed, the ordinary server-tick cap and `player.action_speed_pct`; graph
problem 11/report 12 searches authored action-speed sources. [Breadth validation](docs/breadth-validation.md)
uses the five newly supplied full builds to guide general native admission and further shared
pipelines. Caller-driven [`inspect-build`](docs/build-source-containers.md) preserves
root/container, skill, configuration and [item-source evidence](docs/item-source-and-loading.md)
within the shared bounded XML/lexical subset. With `--with-definitions` or `--data`, it also
reports ordered native item loading and [data-driven formatting](docs/item-formatting.md)
against the selected catalog, stopping at missing parser/assembly operations. Inspection does not calculate effects; native skill admission
remains separate and currently bounded to Spark/Mace. On the legacy command, `--backend pob`
selects the optional reference backend. Native search uses [typed candidate calculation](docs/native-candidate-evaluation.md)
by default, with complete document evaluation available through `--native-evaluation document`.
Search baselines, when a legal initial seed exists, and finalist checks recalculate complete documents.
The developer `search-calibration` command compares complete builds from a required
[caller-supplied catalog](docs/pob-candidates.md); it has no embedded build corpus.
Its fresh finalist checks establish numerical consistency while generic realization and
game legality remain unverified. `extract-tree` exports pinned topology and source/override metadata for broader
mutation work, with live passive coverage and authenticated bounded graph projections. A portable data
crate supplies native class/root/entrance records and numeric game configuration without loading PoB.
Native evaluation, benchmarking and controlled search accept `--data <package.json>`; immutable data is shared
across workers, with dataset identity recorded in results and export companions.
Optional [`extract-game-data`](docs/game-data-extraction.md) regenerates the current native
package from pinned source with a separate extraction-evidence companion.
`benchmark-native` measures prepared or full-document typed evaluation throughput.
Unrestricted joint mutation, full native mechanic coverage, HTML reports and browser
bindings remain unimplemented. See [the runnable experimental workflow](docs/experimental-search.md)
and [search contracts](docs/search-kernel.md).
The [living implementation document](docs/implementation.md) is the progress and resume record;
update it at feature/experiment checkpoints and handoffs.
Start with [the end-state design](docs/design.md), especially its
[confirmed decisions](docs/design.md#confirmed-design-decisions), then the
[implementation resume point](docs/implementation.md#resume-here). See
[PoB integration notes](docs/pob-integration.md) for source-verified seams and open runtime questions.
The [execution and interface design](docs/execution-and-interfaces.md) covers multicore
scheduling, library boundaries, structured results, visualization, and the GUI path.
The [WoW prior-art and product review](docs/prior-art-and-product-review.md) records useful
reference workflows, confirmed product decisions, and fixture-validation status.
The supplied exports are preserved as a [decoded PoE2 minion fixture](tests/fixtures/builds/README.md)
with provenance hashes and structural checks. Its level-96 Sorceress / Disciple of Varashta
build now produces fresh diagnostic outputs. Three skill entries remain unresolved, Full
DPS has no included groups, and upstream startup diagnostics are retained in the result.
The output is not a certified build recommendation; cached values are never the evaluator.

## Getting started

Install Git and Rust through rustup, with a working native linker
(on Windows, Visual Studio Build Tools with the C++ toolchain).
Use the repository-pinned Rust toolchain so local formatting and lint checks match CI.

For the native-only executable (supported profiles are listed in [native backend](docs/native-backend.md)):

```powershell
cargo build -p poe-optimizer-cli --release --no-default-features --locked
cargo run -p poe-optimizer-cli --no-default-features --locked -- evaluate tests/fixtures/calibration/spark-mapping.xml
cargo run -p poe-optimizer-cli --no-default-features --locked -- search-build --problem examples/passive-equipment-search.json --jobs 4 --max-evaluations 1000
cargo run -p poe-optimizer-cli --no-default-features --locked -- search-experimental --problem examples/mace-search.json --jobs 4 --max-evaluations 10
cargo run -p poe-optimizer-cli --no-default-features --locked -- search-experimental --problem examples/mace-class-search.json --jobs 4 --max-evaluations 746
cargo run -p poe-optimizer-cli --no-default-features --locked -- search-experimental --problem examples/mace-resistance-search.json --jobs 4 --max-evaluations 842
cargo run -p poe-optimizer-cli --no-default-features --locked -- search-experimental --problem examples/mace-support-search.json --jobs 4 --max-evaluations 2942
cargo run -p poe-optimizer-cli --no-default-features --locked -- search-experimental --problem examples/mace-local-weapon-search.json --jobs 4 --max-proposals 8192 --max-evaluations 4412
cargo run -p poe-optimizer-cli --no-default-features --locked -- search-experimental --problem examples/mace-actor-search.json --jobs 4 --max-proposals 8192 --max-evaluations 4412
```

For the full development workspace, including optional PoB references and parity tests:

```powershell
git submodule update --init --recursive
cargo run --locked -- --help
cargo run --locked -- --version
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
```

The project selects stable Rust and declares Rust 1.93 as its minimum version. Cargo
builds a pinned LuaJIT runtime and the native UTF-8 module when the PoB feature is enabled; no Lua
executable or upstream Windows DLL is required. The native C compiler is needed in
addition to the Rust linker. CI checks the full workspace on Windows and Linux.

Import and evaluate the preserved original single-build fixture:

```powershell
cargo run --locked -- import tests/fixtures/builds/pobarchives-Dfz36mCq.import.txt
New-Item -ItemType Directory -Force runs | Out-Null
cargo run --locked -- evaluate tests/fixtures/builds/pobarchives-Dfz36mCq.import.txt --timeout-seconds 30 --output runs/example.json --export runs/example.xml
```

The current `example.import.txt` contains five builds, one per line; use the
[corpus intake runner](docs/breadth-validation.md#reproduce-corpus-intake) for that file. Add
`--inspect-build --with-definitions` to retain source skill occurrences, ordered raw item/range
instructions, saved equipment sets, passive-spec jewel references and injected skill/configuration
identity evidence and explicit native item-loading stops alongside independent evaluation results. The runner writes corpus schema 5. `--inspect-configuration`
retains a separate configuration source report. Individual XML/share
inputs can also be inspected with `inspect-configuration INPUT --with-definitions` to look
up settings in the complete injected catalog; `--data PACKAGE` selects custom definitions.
Definition recognition does not evaluate effects. Omitting both options gives source-only
`inspect-configuration INPUT`; it works without PoB and
reports source data separately from unimplemented mechanics.
Single-build commands accept one build document or share string.

`inspect-build INPUT` report schema 3 preserves authored skill sets, groups, gems and selectors,
plus an independent item projection. Item evidence separates exact text/comment/CDATA fragments
from ordered strings and child instructions consumed by the PoB XML reader. It preserves
ModRange order, inactive equipment sets and jewel assignments under their original passive specs.
Source-only inspection does not execute item loading. Equipment resolution and
passive-allocation checks remain separate. See
[item source and loading](docs/item-source-and-loading.md).

Add `--with-definitions` or `--data PACKAGE` for source-bound skill/configuration identity
lookup and ordered native item-loading diagnostics against the complete injected base catalog.
Missing parser, formatting or assembly dependencies stop loading explicitly, preserving the
state reached and later unexecuted instructions. A loading report does not certify complete
item calculations or build support. Ambiguous identities and unprocessed names remain explicit. [Skill inspection](docs/skill-source-and-identities.md) works with the native-only CLI
and does not claim numerical support for recognized skills. Local projection errors remain
separate after the document passes the shared XML/lexical gate; a global failure prevents the
inspection report.

Output files must be new paths; existing files are never overwritten. Without `--output`,
evaluation JSON goes to stdout. `--pob` selects a source directory matching the committed
source manifest. The evaluator validates its source directly and does not run Git at runtime.

List the metric catalog or select an explicit calibration context:

```powershell
cargo run --locked -- metrics
cargo run --locked -- evaluate tests/fixtures/calibration/spark-mapping.xml --options examples/evaluation-options.json --metric player.selected_hit_dps --metric player.life
```

`--metric` is repeatable; use `player.<id>` or `minion.<id>`. With no filter, all declared
actor/metric combinations are returned, including unavailable measurements. Percent values
use percentage points (`75` means 75%). Finite, positive/negative infinity, NaN and unavailable
are distinct. Combined DPS and Full DPS rollups are not registered objective metrics.

The options file accepts one-based socket-group/action indexes and an optional selected
minion action. Encounter overrides set a name, enemy level, boss kind and/or all five incoming
hit components. Omitted fields retain imported assumptions; naming a scenario does not reset
buffs, conditions or enemy settings. The result records requested options and observed
configuration/conditions, which are not yet a complete resolved scenario model. See
[skill coverage](docs/skill-coverage.md) for action provenance and unresolved entries.

Evaluation-report JSON uses schema 3; the PoB worker protocol, controlled-search reports and
native benchmark reports use schema 2. Expanded class/tree search reports use schema 3
with explicit budgets and admission evidence; paid-ascendancy problems use report schema 4.
[Configurable support-loadout problems](docs/support-loadouts.md) use report schema 5;
[local-weapon problems](docs/local-weapons.md) use report schema 6;
[actor-configuration problems](docs/actor-resources.md) use problem 6/report 7;
[passive/equipment problems](docs/passive-equipment-assembly.md) use problem 7/report 8. Synthetic item rolls in
the latter example exercise calculation rules; they do not certify obtainable affix sets.
Results identify backend,
rules/source/adapter fingerprints, observed selection, metric schema/units and coverage.
`--raw` adds the complete PoB diagnostic snapshot as an opaque JSON attachment. Objective code
must use typed measurements rather than inspect raw PoB fields. Unknown fields in options,
unsupported metrics and replaced explicit selections fail instead of silently falling back.

PoB evaluation exports normalized serialization. Native evaluation exports validated source
with supported options applied and stale cached outputs removed. `import` preserves original XML bytes.
Evaluation requires explicit supported build/tree metadata. Current outputs remain diagnostic:
the [independent calibration](docs/calibration-reference.md) covers controlled Spark and attack cases,
not general mechanic support, build legality or search recommendations. Non-damaging actions
and average-damage modes do not yield an invented sustainable DPS objective.

Configure an objective or reassess a saved evaluation:

```powershell
cargo run --locked -- evaluate tests/fixtures/builds/pobarchives-Dfz36mCq.import.txt --objective examples/evaluation-objective.json --output runs/assessed.json
cargo run --locked -- assess runs/assessed.json --objective examples/evaluation-objective.json
```

The example maximizes EHP with three resistance constraints; the supplied build misses the
lightning threshold by four percentage points. Every objective and threshold is configurable.
See [objective assessment](docs/objective-assessment.md) for strict comparisons, units,
unavailability and saved-result behavior. Passing these constraints is not a legality verdict.

Compare complete caller-supplied builds with the optional PoB reference backend:

```powershell
New-Item -ItemType Directory -Force runs | Out-Null
cargo run --locked -- search-calibration --catalog examples/calibration-catalog.json --objective examples/calibration-objective.json --jobs 2 --max-evaluations 5 --output runs/calibration-search.json --export runs/calibration-best.xml
```

The required catalog uses schema 1 with `builds: [{ "id": "...", "path": "..." }]`;
relative paths resolve from the catalog file and accept PoB XML or share codes. The example
manifest explicitly selects the four original regression fixtures. Supply another manifest
to compare other complete documents. Report schema 2 separates exact requested XML hashes
from observed PoB summaries, selected actions/configuration, coverage and normalized-export
hashes. Export saves the exact requested XML after fresh numeric verification; it does not
certify that PoB preserved every requested mechanic or that the build is legal. See the
[catalog contract and bounds](docs/pob-candidates.md#supplied-catalog-cli).

The [calculation boundary decision](docs/calculation-boundary.md) explains replacing the PoB
backend without changing callers. See the [native engine design](docs/native-engine.md) for
translation/parity gates and browser requirements. To check portable libraries:

```powershell
rustup target add wasm32-unknown-unknown
cargo check -p poe-optimizer-core -p poe-optimizer-engine -p poe-optimizer-data -p poe-optimizer-import -p poe-optimizer-native --lib --target wasm32-unknown-unknown --locked
```
## Repository

- `src/`: thin CLI and hidden worker protocol entry point.
- `crates/poe-optimizer-core/`: backend-neutral evaluation contracts, options, metrics and coverage.
- `crates/poe-optimizer-engine/`: portable native calculation kernels and differential tests.
- `crates/poe-optimizer-data/`: authenticated portable tree models, projections and native passive/equipment data. Native evaluation now accepts [injectable data packages](docs/native-data.md); broader source/tree update compatibility remains in progress.
- `crates/poe-optimizer-native/`: strict native document profiles, preparation and typed backend.
- `crates/poe-optimizer-import/`: portable bounded decoding, preflight and controlled materialization.
- `crates/poe-optimizer-pob/`: optional reference host, source extraction, verification and supervision.
- `crates/poe-optimizer-lua-utf8/`: static Unicode module, native sources and provenance.
- `docs/design.md`: end-state scope, architecture, objectives, search, and acceptance criteria.
- `docs/implementation.md`: living delivery plan, current status, checks, and session resume point.
- `docs/pob-integration.md`: findings from the pinned upstream source.
- `docs/execution-and-interfaces.md`: parallel runtime, core APIs, artifacts, and frontend plan.
- `docs/prior-art-and-product-review.md`: cited references, prioritized gaps, and decision status.
- `examples/evaluation-objective.json`: runnable scalar objective and hard constraints.
- `examples/evaluation-options.json`: runnable diagnostic selection and encounter overrides.
- `examples/calibration-catalog.json`: explicit four-fixture input manifest for the developer comparison command.
- `examples/objective.toml`: illustrative future configuration, not a supported CLI input.
- `vendor/path-of-building-poe2/`: unmodified Git submodule.
- `local/`, `runs/`: ignored locations for private build inputs and generated results.

## Upstream dependency

[PathOfBuildingCommunity/PathOfBuilding-PoE2](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2)
is pinned by the Git submodule entry. The initial inspected revision is
`3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`, downloaded from the default `dev` branch.
This revision boots and calculates the supplied fixture through the embedded host.
Mechanic coverage and independent numerical parity remain under validation.

```powershell
git submodule status
git -C vendor/path-of-building-poe2 rev-parse HEAD
```

The initial clone is shallow to keep setup small. Fetch more upstream history when
needed with `git -C vendor/path-of-building-poe2 fetch --unshallow`.
Ordinary setup must use the recorded revision, not `git submodule update --remote`.
An upstream upgrade should explicitly select a revision, run evaluator parity
checks and review translated native stages, then commit the changed submodule pointer.

Upstream licensing and third-party notices remain in
[its LICENSE.md](vendor/path-of-building-poe2/LICENSE.md). This scaffold does not
choose a distribution license for the new project; select one before a public release.
