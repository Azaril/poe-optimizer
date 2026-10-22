# poe-optimizer

Experimental Rust tooling for configurable Path of Exile 2 build optimization,
with potential PoE1 support later. Import a build, choose allowed changes and
locks, configure objectives/constraints and scenarios, then search for verified
best-found alternatives using a native parallel evaluator.

The first usable optimizer targets joint class/ascendancy, passive, equipment,
support-gem and supporting-skill search, including 1..N required skills and exact
item locks. Goals are user-defined; damage, resistance and EHP are examples.
Configurable 5–30 minute runs target both bossing and mapping scenarios.

## Current status

**No supplied original build yet completes native evaluation (0/5).** The closest
is Original05 Sniper. Typed native rule/effect components exist, but a native build
optimizer is not wired up. The current `search-calibration` command ranks supplied
PoB builds; it is not that optimizer.

The October 22 [architecture review](docs/architecture-review-2026-10-22.md) is
adopted. Before adding further mechanic content, migrate to readable `.rule`
formulas and KDL manifests, one checked IR, reproducible releases and an interned
runtime package. Consolidate tests and remove redundant legacy paths. **Migration
has started: complete program compilation, formatting, and name/membership KDL
loading exist; release publication and the binary runtime remain ahead.** The first
complete unchanged build must run exclusively through the new format and native
path; then native search and broader
build validation follow.

Current default Data/Engine dependencies still enable the legacy source-compatibility
stack and embed its snapshot. Omitting `pob` does not yet remove that Rust legacy
code. Actual LuaJIT hosting is optional through `mlua`; PoB will remain an optional
offline extraction/parity reference, outside the native candidate loop.

Start with the [documentation index](docs/README.md),
[target architecture](docs/domain-architecture.md),
[implementation plan and resume point](docs/implementation.md), or the short
[data/execution overview](docs/data-and-evaluation-overview.md).

## Development

Use the pinned Rust toolchain in `rust-toolchain.toml` and Cargo's locked dependency
set. Windows builds require the appropriate native compiler toolchain. New core,
data tooling and tests use Rust; shell scripts may orchestrate commands.

```powershell
cargo run --locked --bin poe-optimizer -- --help
cargo run --locked --bin poe-optimizer -- evaluate-owned --help
```

For optional PoB tools, initialize the pinned submodule and build with `pob`:

```powershell
git submodule update --init --recursive
cargo run --locked --features pob --bin poe-optimizer -- --help
```

`evaluate-owned` is the existing native component/evaluation entry point;
`evaluate` under the `pob` feature runs the reference backend. There is not yet a
clean-clone command that reproduces the latest full data release; fixing that is
an explicit migration gate. Do not substitute the tracked historical `current/`
baseline for the latest local release or assume a complete build will evaluate.
The offline `rule fmt` command is available; manifest publishing remains planned.

The existing suite is expensive. Consolidation reduces Core, Data, Engine,
Search and Import from 304 integration binaries to twelve; CLI and reference
suites still need consolidation. The plan also removes source-text identity gates
and duplicate fixtures before resuming content. Follow the current plan for scoped
validation; a successful push or local check is not evidence of a passing hosted suite.

Register new test modules in the explicit suite roots: `tests/integration.rs` for
Core/Search and `tests/owned.rs` for native Data/Engine and owned Import contracts.
Import keeps its large normalization suite isolated. Adding a test file
alone no longer registers it in those crates. The temporary `legacy` suites retain
migration checks and are scheduled for removal with the old runtime.

The `poe-optimizer-authoring` library parses and lowers complete `.rule` programs
to the existing IR, resolves exact schema symbols and raises IR back to source.
Its normal dependencies are Core and the official `kdl-rs` parser; it is not a runtime
dependency. The separate `poe-author` executable formats one program to stdout or
checks formatting:

```text
cargo run --locked -p poe-optimizer-authoring -- rule fmt path/to/mechanic.rule
cargo run --locked -p poe-optimizer-authoring -- rule fmt --check path/to/mechanic.rule
```

Use `-` as the path to read stdin. Input files are unchanged. Check mode exits
0 when formatted, 1 when formatting differs, and 2 on syntax, argument or I/O
errors. Formatting preserves names, numeric spelling and comments; it does not
validate rule types, ownership or complete coverage. Name/membership KDL loading
and bounded packet assembly are library APIs; broader declaration manifests,
semantic checking/publication commands and release cutover remain ahead.
Run focused checks with `cargo test -p poe-optimizer-authoring --locked --all-targets`.

## Boundaries

- Game data and build inputs are injected; no production fixture/name-specific
  paths. One generic Actor/occurrence model supports players, enemies and minions.
- Direct native Rayon evaluation, shared immutable data and private worker scratch;
  no subprocess per native candidate. CLI first, shared web/desktop UI later.
- No SQLite/DuckDB/ORM. UI search can use loaded definitions/indexes.
- PoB XML/share codes remain interchange at Import; PoB Lua/UI behavior does not
  define the native model. Keep useful parity cases while deleting obsolete code.
- No backward compatibility is required for these unshipped formats. Rebuild data
  and reimport through the one current path.

Source/data notices and a distribution license must be resolved before a public
binary/data release. See [product design](docs/design.md) for requirements,
validation criteria and deferred decisions. Existing reference material and
historical commands remain available in Git, not as an alternative active design.
