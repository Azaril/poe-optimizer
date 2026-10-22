# Data acquisition and evaluation overview

Updated 2026-10-22. The [architecture](domain-architecture.md) defines the accepted
end state; [implementation](implementation.md) records what is delivered. This
page explains the distinction. **The DSL migration is not implemented yet, and
0/5 supplied original builds complete native evaluation.**

## What runs today

Three different systems exist in the baseline, and their names can obscure that:

| System | What executes | Status |
| --- | --- | --- |
| Owned native evaluator | Rust checks and interprets typed SSA rules, then resolves/executes exact occurrence effects and metrics | The foundation to retain and improve; component coverage exists, full originals remain blocked |
| Source-compatibility stack | Rust implements Lua-shaped parsing/control/data semantics; legacy Data/Engine features and the embedded snapshot are enabled by default | To remove with its remaining consumers; it is not the desired long-term rule language |
| Actual PoB host | `mlua` embeds LuaJIT, loads pinned PoB Lua and provides host shims in isolated reference processes | Optional extraction/reference tool to retain |

A Rust implementation of Lua-like behavior is different from running LuaJIT.
Both exist today. Neither should be confused with the owned typed-rule VM.
The review found default dependencies still pull the legacy stack into binaries;
absence of the `pob` feature alone does not prove the native distribution is clean.

Current Import also contains much of the offline release toolchain. Hand-authored
JSON describes rule IR and catalog bindings, while many test-support functions
perform publication. The latest validated release is local under ignored `runs/`;
the tracked directory called `current/` is a historical baseline. That is a
reproducibility defect, not a supported distribution arrangement.

`evaluate-owned` is the current native CLI path. The `evaluate` command is the
optional reference backend, available when Cargo builds with `--features pob`. Existing
`search-calibration` ranks a finite supplied set through PoB; it is not a native
build optimizer. New DSL commands below are planned, not available commands.

## What replaces it

1. **Offline Rust acquisition** reads pinned upstream tables and, when necessary,
   executes Lua through the optional PoB host. It emits owned declarations and
   separate provenance. Upstream callbacks and UI objects do not cross this boundary.
2. **Readable source** uses named definitions, `.rule` formulas and KDL manifests.
   This is where authors specify mechanics, ownership, inputs, queries and fixtures.
3. **Authoring compilation** resolves symbols and lowers to the existing checked
   SSA IR. Its checker validates units, permissions, dependencies and operations.
   A compact canonical artifact is reviewable and reproducible.
4. **Packing** creates a bounded binary runtime package with interned IDs and
   separate Import recipes. Publication is a production Rust tool driven by a
   tracked manifest, not a chain of test functions or developer-local directories.
5. **Import** decodes a user's build and applies the selected conversion recipes
   once. It produces the same owned model that a future UI can author directly,
   plus explicit unresolved diagnostics and an external-source sidecar.
6. **Native evaluation** prepares a reusable structural plan, binds indexed
   candidate values and runs with private worker scratch. Typed metrics and
   requirement assessment feed the CLI/search/UI. No Lua or subprocess executes
   for a native candidate.

XML remains an external build interchange format; it is not the runtime definition
database. SQLite/DuckDB/ORM are outside the design. Loaded Rust data/indexes can
support later filtering/autocomplete. The new binary encoding is an execution
optimization, not another semantic model.

## Conditional logic

A `.rule` formula lowers offline into the existing typed operations. The native
VM executes that checked graph; it never parses `.rule`, KDL or Lua during search.
Names and source spans improve authoring/errors/explanations while dense handles
keep arithmetic cheap. Programs retain exact occurrence permissions and explicit
coverage. A short formula cannot grant an otherwise unauthorized read or writer.

Lazy conditions preserve unchosen branches and their failures. Numeric units,
percent conversion, operation order, finite checks and explicit unsupported
domains are checked semantics. Migration must preserve them exactly. Proposed
rounding/failure-lattice improvements are separate behavior changes, not an
excuse to alter results while converting syntax.

Saved conditional settings are not executable logic. Import maps them to typed
build/usage/scenario inputs with explicit ownership. Native rules derive resulting
conditions from those inputs and the candidate. Necessary XML or item-text quirks
remain at Import; incidental Lua truthiness and UI cache state do not become
public native types.

## Runtime reuse and parallelism

The engine shares immutable datasets and structural plans through its library API.
Each worker owns scratch. Indexed parameters avoid recompiling a plan for every
value-only candidate, but dependency declarations must identify values that can
change support eligibility, generated sources, readiness or coverage. Such changes
invalidate the affected structure. Cache-state independence is tested cold/warm;
it does not require recomputing immutable data every time.

Native search uses Rayon and bounded queues. A proposal sequence controls result
integration in deterministic count-budget runs. The optional PoB oracle may use
isolated processes for validation; that cost is outside the native search loop.

## Migration and first-build gate

[Phases R0-R4](implementation.md#delivery-plan-and-gates) make tests usable,
separate authoring, establish reproducible data, implement DSL/manifests and the
runtime library, then delete obsolete paths. No additional mechanic content is
added on the old format. Useful laws and original raw builds survive migration;
source-file hash gates, redundant receipts and per-packet publisher tests do not.

R5 then closes the nearest unchanged build through the new public path and checks
complete coverage, requirements and stable numerical references. Original05 Sniper
is the starting target. Its five input issues are only the current admission
frontier; resolving them alone will not finish its numerical/legality support.
After that first working build, implement one real native optimizer axis and then
expand to the remaining originals and the required joint optimizer.

For pre-migration implementation details, see the
[overview at fa49fcea](https://github.com/Azaril/poe-optimizer/blob/fa49fcea32f0413cde5f5983e179c34e6c7acbdf/docs/data-and-evaluation-overview.md).
It is historical evidence, not a second target design.
