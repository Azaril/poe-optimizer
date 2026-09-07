# poe-optimizer

An experimental Rust project for constraint-driven Path of Exile 2 build optimization,
with potential Path of Exile 1 support later. The intended workflow is to import a
build, choose which build decisions may change, fix combat assumptions, and configure goals,
then search for good legal alternatives using Path of Building's calculations.

Goals are user-configurable. Damage, resistance, EHP, and skill-selection examples are
illustrative; users choose objectives, constraints, and preferences from an extensible
metric catalog. The first search domain keeps skills and gear fixed while reallocating
passives; richer scoring policies and search domains are separate design stages.

The planned engine uses reusable Rust core libraries, Rayon for parallel Rust work,
and isolated PoB workers under shared CPU/memory limits. The CLI comes first, with
structured run data and offline HTML reports; a later GUI can use the same APIs,
with Tauri as a candidate.

**Status:** repository scaffold and design proposal. The CLI prints project information;
it does not evaluate or optimize builds yet.

Start with [the design proposal](docs/design.md), especially its
[alignment decisions](docs/design.md#decisions-to-align-on). See
[PoB integration notes](docs/pob-integration.md) for source-verified seams and open runtime questions.
The [execution and interface design](docs/execution-and-interfaces.md) covers multicore
scheduling, library boundaries, structured results, visualization, and the GUI path.

## Getting started

Install Git and Rust through rustup, with a working native linker
(on Windows, Visual Studio Build Tools with the C++ toolchain).

From this repository:

```powershell
git submodule update --init --recursive
cargo run --locked -- --help
cargo run --locked -- --version
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
```

The project selects stable Rust and declares Rust 1.93 as its minimum version.
The bootstrap was checked with Rust 1.93.0 on Windows. There are no Rust library
dependencies yet, and compiling the scaffold does not require Lua or the submodule.
There are no calculation or optimization tests yet; the test command verifies the
starter target builds. The included CI workflow checks the Rust scaffold on Windows
and Linux when the project is hosted on GitHub.

## Repository

- `src/`: minimal CLI scaffold; implementation will follow design alignment.
- `docs/design.md`: proposed scope, architecture, objectives, search, and milestones.
- `docs/pob-integration.md`: findings from the pinned upstream source.
- `docs/execution-and-interfaces.md`: parallel runtime, core APIs, artifacts, and frontend plan.
- `examples/objective.toml`: illustrative future configuration, not a supported CLI input.
- `vendor/path-of-building-poe2/`: unmodified Git submodule.
- `local/`, `runs/`: ignored locations for private build inputs and generated results.

## Upstream dependency

[PathOfBuildingCommunity/PathOfBuilding-PoE2](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2)
is pinned by the Git submodule entry. The initial inspected revision is
`3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`, downloaded from the default `dev` branch.
This is an inspected source snapshot, **not yet a runtime-validated evaluator**.

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
choose a distribution license for the new project; select one before publishing.
