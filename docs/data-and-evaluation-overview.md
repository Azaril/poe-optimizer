# Data acquisition and evaluation overview

Updated 2026-10-22. The [architecture](domain-architecture.md) defines the accepted
end state; [implementation](implementation.md) records what is delivered. This
page explains the distinction. **Migration has started, but release-wide DSL migration
and runtime cutover remain ahead; 0/5 supplied originals complete natively.**

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
JSON describes rule IR and catalog bindings, while many historical test-support
functions perform publication. A production manifest builder now composes the
final constituent files directly. The tracked
[`migration-source/release.kdl`](../data/owned/poe2/3887ae68/migration-source/release.kdl)
and compressed inputs capture the previously local endpoint. Rust tests and the
production CLI reproduce all seventeen constituent artifacts and the reference
receipt exactly, without local predecessor inputs or test staging helpers.
The tracked directory called `current/` remains a historical baseline. Neither
the capture nor its compression completes the DSL or runtime migration.

The new `poe-optimizer-authoring` crate parses bounded program declarations,
typed reads, ordered `let` calculations and guarded effects. A checked symbol
registry resolves readable names to existing exact definition/slot addresses.
Named intermediates share one rule-node graph without duplicating calculations
or inventing reads. Every current IR expression family has source syntax,
including checked ranges, exact integer-table lookups, choice identities and the
atomic ordinary-timing calculation. The compiler preserves existing native
operations; it does not implement a second calculation engine.

For example, this is a complete source program given the named definitions:

```text
program "add-life" context actor {
    read "base-life" as base: quantity(unit.life) = stat(current, stat.base_life);
    let total = base + quantity(20, unit.life);
    effect "life" = derive(current, stat.life, total);
}
```

The source does not declare its owning definition or claim complete membership.
Those belong to the manifest. Lowering produces an unchecked `RuleProgram`,
which still needs the existing Engine's complete package checks before execution.
Authoring uses Core, the official `kdl-rs` 6.5.0 parser and ordinary Rust
serialization/compression libraries. Its CLI uses the shared Host crate for
filesystem publication. The Import dependency is temporary: naming reuses its
existing allocator and release construction reuses its typed assembler, which
also brings Import's legacy features into this offline tool.
There is no reverse Import-to-Authoring dependency. No runtime package depends
on Authoring, and it does not evaluate formulas.

The reverse compiler now emits readable source from existing IR, with an explicit
node mapping for exact graph comparisons. It preserves sharing, unused nodes,
operand order and the effect ledger, and checks structural references and bounds.
The existing Engine still checks semantic authority and executes the result.
Source formatting preserves exact tokens, authored names and comment bodies while
normalizing structural whitespace. It is idempotent under the same resource limits;
generated output must fit the input limit as well as the output limit. Name and
program-membership KDL manifests now load through a bounded library API. Names
resolve exact schema definitions/slots, and packets append explicitly declared
programs to existing owners after a complete structural preflight. The containing
package still requires Engine validation. Source labels are injected text labels,
not paths that the library opens. Broader definition declarations and release-wide
DSL conversion remain pending. The new release manifest described below composes
existing constituent DTOs; it does not yet integrate allocation and DSL packets
into a single publication transaction.

Stable naming now has an in-memory staged transaction API. `NamedIdRegistry`
wraps the single existing Import allocator; `NamesInput` stores its exact registry
digest plus sorted typed bindings. Transactions leave their base unchanged and
allocate in definition-name order followed by slot-name order. Exact kind/owner
replays reuse identities; aliases must target identities already active in the
base. The predecessor digest covers alias-only changes too. Retired bindings stay
reserved in history and are omitted from executable symbols. Resolving active
names still requires the actual schema, and the complete rule package still
requires Engine validation.

No filesystem compare-and-swap or KDL allocation command is implemented by that
API. The release builder does not publish named-allocation transactions.
`load_names` still loads raw aliases to known schema
addresses. Full R1 extraction must separate runtime registry/role data from the
offline toolchain and replace the runtime release loader that currently
reassembles authoring inputs. This library step does not publish a release or
migrate every existing program.

KDL is maintained offline source: file bytes are limited before parsing and
retained structure is checked afterward. These checks are not a guarantee of
bounded parser recursion for hostile input. KDL is not parsed during native
evaluation or used to ingest uploaded builds.
One upstream syntax limitation is known: `root { /- omitted "value"; kept; }`
is valid KDL but rejected by `kdl-rs` 6.5.0; the same parser path remains in
6.7.1. Newline-terminate commented-out nodes instead. Our adapter does not rewrite
the source or retry it through another parser. This affects authoring syntax,
not rule calculations or imported builds.
Component round-trips do not establish complete-build parity or migrate the release.

`evaluate-owned` is the current native CLI path. The `evaluate` command is the
optional reference backend, available when Cargo builds with `--features pob`. Existing
`search-calibration` ranks a finite supplied set through PoB; it is not a native
build optimizer. The separate offline `poe-author rule fmt [--check] <path|->`
command is available; it writes formatted source to stdout or checks formatting
without changing the file. It validates syntax only. Release construction now has
the separate production command below; other DSL developer commands remain planned.

## Production release composition during migration

`poe-author release build <release.kdl> --output <new-directory>` reads a declared
final composition and publishes a checked release. For the captured full source:

```text
cargo run --locked -p poe-optimizer-authoring --bin poe-author -- release build data/owned/poe2/3887ae68/migration-source/release.kdl --output rebuilt-release
```

The output directory must not exist and its parent must exist. The retained
full-source rebuild and reference-receipt comparison pass. The command reuses
the existing typed release assembler; it
does not call test staging helpers, replay the old publication chain or provide a
second semantic checker.

The closed manifest requires registry, schema, rules, routing, mapping, roles,
normalization, rewards, item rules and item-source policy. Tree normalization and
provenance are optional. Metrics can be supplied alone. A support evaluation
group requires metrics plus stages, preparation, input bindings and receiving;
support outputs are optional only within that group. Named query sets retain the
order authored in the manifest. Unknown fields, duplicate declarations or source
paths, duplicate query names and incomplete groups are errors.

Each constituent is a current typed JSON DTO, either plain JSON or explicitly
declared `encoding="gzip"`. This is temporary migration input before readable
DSL/declaration sources and the compact compiled package replace it. The library
receives injected bytes; the CLI resolves source paths within the manifest
directory and bounds file reads. Separate aggregate limits cover encoded and
decoded bytes, with additional manifest, source-count and typed assembly limits.
Gzip accepts exactly one complete member and rejects trailing bytes or additional
members. Compression changes transport size, not mechanics or decoded limits.

Optional `expected_input` asserts equality with the assembled release-input
digest. It is an exact reproduction check, not a coverage certificate. Both CLI
hosts call the same `poe-optimizer-host` publication implementation, which writes
and syncs staged files before a no-clobber directory rename on Windows/Linux.
Host contains no game, Import or Authoring dependencies. The native CLI still
loads releases by reassembling authoring inputs; extracting its immutable loader
remains required before runtime cutover.

The captured full source has no evaluation bundle and does not add rule coverage.
**No supplied original is complete natively (0/5).** Rebuilding its release is a
reproducibility step toward the new-format first build, not that build's completion.

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
