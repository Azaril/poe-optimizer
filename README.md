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

**Architecture correction (2026-09-14):** the target is a project-owned semantic build model
and domain-rule package compiled offline from PoB, with independent UI, import, evaluation
and search layers. PoB remains an optional parity oracle. Existing Spark/Mace profiles and
source-shaped native loaders are legacy paths being retired; none of the
five supplied real builds yet completes natively. See the [controlling design](docs/domain-architecture.md),
[reviewed owned-input contract](docs/owned-build-contract.md),
and [migration plan](docs/architecture-migration.md). The capabilities below describe the
current experimental implementation, not the target architecture.

**Status:** experimental import and evaluation CLI with switchable native and optional PoB
backends. Numerical evaluation still uses PoB documents. A new
`check-owned-input INPUT --canonical-output OUTPUT` command checks directly authored owned
build/inventory/scenario/query/request structure through portable core APIs, without XML or
a data package. `check-owned-schema INPUT --canonical-output OUTPUT` validates a supplied
owned definition schema package and reports its canonical content identity. Both commands
work in a native-only build; neither claims build binding, legality or numerical coverage.
`check-owned-rules INPUT --definitions SCHEMA [--probe FACTS]` checks the new
owned rule format, compiles its typed effects, and optionally evaluates explicit component
facts without PoB. This is the shared library boundary for real effect conversion and
future build-plan integration; it does not yet calculate an owned build. See
[owned rule components](docs/owned-rules.md).
`check-owned-draft INPUT --selection SELECTION --owned-output OUTPUT` validates partial
owned authoring state and finalizes explicit independent presets through the same Core
APIs. Pending selections retain all query rows and cannot write a complete request;
see [owned drafts](docs/owned-drafts.md). This command also works without PoB and does
not calculate, bind definitions or certify legality.
Owned project/draft persistence now uses version 2. Allocation presets can contribute
socket equipment and choice presets can contribute rewards independently; older envelopes
are rejected explicitly. Concrete build/request payloads remain unchanged.
`normalize-owned` converts caller-supplied XML/share codes through injected owned schema,
mapping, role, reward and normalization policy artifacts into partial owned drafts. It preserves independent
alternatives and unresolved semantics; it does not yet produce a complete original build
request or calculate. See [owned normalization](docs/owned-normalization.md).
The existing numerical CLI accepts PoB share codes/XML, explicit skill/action and encounter options,
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
The obsolete `search-experimental` command, finite Mace catalog/candidate API and dedicated
benchmark have been removed. Old problem files are not silently rerouted. The surviving
`search-build` is also profile-limited and will be replaced through the
[retirement plan](docs/legacy-retirement.md). Shared actor, defence, item and timing kernels
and independent numerical/reference fixtures remain available for that migration.
Caller-driven [`inspect-build`](docs/build-source-containers.md) preserves authored build
containers and reports available definitions and explicit preparation gaps; inspection is
not a complete evaluation. Native graph search uses prepared inputs by default and retains
`--native-evaluation document` for complete-document comparison.
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
bindings remain unimplemented. See [the retained graph workflow](docs/passive-equipment-assembly.md)
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

Add [`--with-instances`](docs/build-instances.md) to retain an owned source-to-instance map
with a fresh build lineage, distinct saved sets/entries/item uses and explicit projection
failures. This is source identity, before active-set resolution or native evaluation.

Add `--with-definitions` or `--data PACKAGE` for source-bound skill/configuration identity
lookup and ordered native item-loading diagnostics against the complete injected base catalog.
Native formatting, ordinary modifier parsing, defence-header state and base buff loading
use the selected data. Pending callback, conversion or assembly dependencies stop loading explicitly, preserving the state reached and later
unexecuted instructions. A loading report does not certify complete
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

Selected saved alternatives can be inspected with `inspect-build <single-build-input> --with-view`.
This reports independent set choices, instance/definition bindings and outstanding preparation
stages. See [selected views](docs/selected-views.md); it does not calculate a complete build.

Use `prepare-build <single-build-input>` for the native evaluator's structured preparation
result and source-linked missing stages. It performs no calculation. See
[the native preparation API](docs/native-preparation.md).
