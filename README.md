# poe-optimizer

An experimental Rust project for constraint-driven Path of Exile 2 build optimization,
with potential Path of Exile 1 support later. The intended workflow is to import a
build, choose which build decisions may change, set encounter assumptions, and configure goals,
then search for good legal alternatives using Path of Building's calculations.

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

The planned engine uses reusable Rust core libraries, Rayon for parallel Rust work,
and isolated Rust workers hosting PoB through `mlua` with the LuaJIT backend, under
shared CPU/memory limits. The CLI comes first, with
structured run data and offline HTML reports; a later GUI can use the same APIs,
with Tauri as a candidate.

**Status:** experimental import and evaluation CLI. It decodes PoB share codes/XML,
loads the pinned PoB engine through `mlua`, and returns fresh raw player/minion outputs
plus an optional PoB XML export. Each request uses an isolated process with a deadline.
Optimization, certified metric mappings, persistent worker pools, and reports are not
implemented yet. The [living implementation document](docs/implementation.md) is the
progress and resume record; update it at feature/experiment checkpoints and handoffs.
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

From this repository:

```powershell
git submodule update --init --recursive
cargo run --locked -- --help
cargo run --locked -- --version
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
```

The project selects stable Rust and declares Rust 1.93 as its minimum version. Cargo
builds a pinned LuaJIT runtime and the native UTF-8 module from vendored source; no Lua
executable or upstream Windows DLL is required. The native C compiler is needed in
addition to the Rust linker. CI checks the full workspace on Windows and Linux.

Import and evaluate the supplied fixture:

```powershell
cargo run --locked -- import example.import.txt
New-Item -ItemType Directory -Force runs | Out-Null
cargo run --locked -- evaluate example.import.txt --timeout-seconds 30 --output runs/example.json --export runs/example.xml
```

Output files must be new paths; existing files are never overwritten. Without `--output`,
evaluation JSON goes to stdout. `--pob` selects a source directory matching the committed
source manifest. The evaluator validates its source directly and does not run Git at runtime.

Snapshots identify the runtime, source/adapter fingerprints, selected actors and raw PoB
field names. Non-finite measurements are listed separately; unknown skill entries and
upstream diagnostics remain visible. Export is PoB's normalized serialization, while the
import command preserves the original XML bytes. Evaluation requires a supported explicit
Build section and tree specification; incomplete/ambiguous imports fail instead of using
PoB defaults. No claim of independent numerical parity,
full mechanic support, build legality or optimized results is made by this diagnostic command.

## Repository

- `src/`: thin CLI and hidden worker protocol entry point.
- `crates/poe-optimizer-core/`: reusable evaluation/process contracts.
- `crates/poe-optimizer-pob/`: bounded import, source verification, mlua host and supervision.
- `crates/poe-optimizer-lua-utf8/`: static Unicode module, native sources and provenance.
- `docs/design.md`: end-state scope, architecture, objectives, search, and acceptance criteria.
- `docs/implementation.md`: living delivery plan, current status, checks, and session resume point.
- `docs/pob-integration.md`: findings from the pinned upstream source.
- `docs/execution-and-interfaces.md`: parallel runtime, core APIs, artifacts, and frontend plan.
- `docs/prior-art-and-product-review.md`: cited references, prioritized gaps, and decision status.
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
checks once they exist, and commit the changed submodule pointer.

Upstream licensing and third-party notices remain in
[its LICENSE.md](vendor/path-of-building-poe2/LICENSE.md). This scaffold does not
choose a distribution license for the new project; select one before a public release.
