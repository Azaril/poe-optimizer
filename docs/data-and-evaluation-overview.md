# Data acquisition and evaluation overview

Updated 2026-10-24. The [architecture](domain-architecture.md) defines the accepted
end state; [implementation](implementation.md) records what is delivered. This
page explains the distinction. **The tracked DSL program and named query cutovers are validated;
broader declaration and runtime migration remain ahead. 0/5 supplied
originals complete natively.**

## What runs today

Three different systems exist in the baseline, and their names can obscure that:

| System | What executes | Status |
| --- | --- | --- |
| Owned native evaluator | Rust checks and interprets typed SSA rules, then resolves/executes exact occurrence effects and metrics | The foundation to retain and improve; component coverage exists, full originals remain blocked |
| Source-compatibility stack | Rust implements Lua-shaped parsing/control/data semantics; legacy Data/Engine features and the embedded snapshot are enabled by default | To remove with its remaining consumers; it is not the desired long-term rule language |
| Actual PoB host | `mlua` embeds LuaJIT; bounded static-data constructors run in process, while full build/reference evaluation uses isolated processes and host shims | Optional extraction/reference tool to retain |

A Rust implementation of Lua-like behavior is different from running LuaJIT.
Both exist today. Neither should be confused with the owned typed-rule VM.
The review found default dependencies still pull the legacy stack into binaries;
absence of the `pob` feature alone does not prove the native distribution is clean.

Static acquisition does not require the full build/UI evaluator. For example,
final skill-definition facts reuse the existing bounded original data constructor,
then project exact source identities and typed optional values after every module
has loaded. The offline evidence preserves absence separately from explicit false.
Reviewed KDL declarations and DSL rules supply native meaning; no Lua table,
truthiness rule or source-field spelling becomes a native calculation contract.

Current Import also contains much of the offline release toolchain, and many
historical test-support functions still construct and publish IR directly. The
normal production builder now requires program-free rule metadata and takes
program bodies from tracked semantic KDL names, membership packets and `.rule`
programs. It rejects embedded
program members even when no packets are declared. The tracked
[`migration-source/release.kdl`](../data/owned/poe2/3887ae68/migration-source/release.kdl)
uses that source contract and passes complete graph/placement preservation checks.
Schema, tables, coverage and other metadata remain JSON during R1-R4. The tracked
directory called `current/` remains a historical baseline. Neither the program
source cutover nor compression completes declaration migration, removal of the
old Rust publication paths, or the runtime/authoring boundary cleanup.

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
package still requires Engine validation. Packet APIs receive injected source
labels and text; the release CLI resolves the declared files. The release builder
composes all declared packets before its single checked publication transaction.
Broader definition declarations remain pending; typed Stat declarations now use
the same manifest and publication transaction.

The maintained compiler supports [named contribution-query authoring](authoring-contribution-queries.md).
All 117 current queries are authored in five bounded KDL fragments, including the
three new revival queries. KDL describes exact
contribution channels, producers, groups, reducers, ordering and coverage. They
resolve against the final schema after Stat allocation, using each fragment's
own aliases plus the current declarations. A formula and its query can therefore
refer to a newly declared Stat in one transaction without predicting its ID or
publishing the ledger first. Lowering produces the existing typed query DTOs;
the ordinary storage and Engine validators still check the complete graph.
The 73-name ledger preserves its prior 70 bindings and appends three generic
Skill/Actor channels through the existing shared `declarations/mechanics.kdl`
component. The [revival producers](../data/owned/poe2/3887ae68/revival-producers/README.md)
use those channels for Sniper's intrinsic true flag and twelve-second override,
plus three independent Player passive speed contributions. Exact source and
recipient ownership remain explicit. Incoming membership, numeric ordering and
final revival duration remain incomplete; these component facts do not complete
Sniper's owner or make an original build executable.

Stable naming now has an in-memory staged transaction API. `NamedIdRegistry`
wraps the single existing Import allocator; `NamesInput` stores its exact registry
digest plus sorted typed bindings. Transactions leave their base unchanged and
allocate in definition-name order followed by slot-name order. Exact kind/owner
replays reuse identities; aliases must target identities already active in the
base. The predecessor digest covers alias-only changes too. Retired bindings stay
reserved in history and are omitted from executable symbols. Resolving active
names still requires the actual schema, and the complete rule package still
requires Engine validation.

The allocator itself does no filesystem work. `load_names` still loads raw aliases to known schema
addresses. The maintained library compiler stages typed Stat declarations from
an explicit persisted ledger and returns successor names only after complete
release validation. The CLI publishes an immutable authoring checkpoint containing
`checkpoint.json`, `names.json` and a separate closed `runtime/` directory in one
atomic no-replace transaction. Bootstrap must be explicit; resumed builds require
the predecessor path and expected checkpoint identity. The full runtime and name
bindings are revalidated, including when no new declarations are present. The
receipt commits exact encoded source bytes separately from runtime identity.
There is no separate allocation command, compatibility layout or mutable-head
update. Future mutable-head promotion still requires actual stored-predecessor CAS.
Full R1 extraction must still separate the maintained offline toolchain from
runtime registry/role data. The runtime reader now retains checked packages
directly; it no longer reassembles publication inputs.

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

`poe-author release build` reads a declared final composition and publishes a
checked authoring checkpoint. For the evolving tracked source, first extract and
verify the persisted allocation predecessor using the
[canonical checkpoint reference](../data/owned/poe2/3887ae68/authoring-genesis.md),
then run:

```text
cargo run --locked -p poe-optimizer-authoring --bin poe-author -- release build data/owned/poe2/3887ae68/migration-source/release.kdl --predecessor canonical-checkpoint --expected-predecessor 71ff4429b7fcfb4f75e45d9c58df878453cb25428542d3531074fc08fccbba91 --output rebuilt-release
```

The output directory must not exist and its parent must exist. Runtime consumers
use `rebuilt-release/runtime`. Later builds likewise use a checked predecessor
path and its expected checkpoint identity, with a new output directory. Bootstrap
is reserved for an explicitly new allocation history; it is not the retained
source rebuild command. Successful publication prints the identity to stderr and the
checkpoint receipt JSON to stdout. Optional `stat-declarations "definitions.kdl"
names="declaration-names.kdl"` declares new Stats in the same bounded inventory.
The completed program-source cutover has complete corpus and historical artifact
gates. The named-query syntax migration preserved the prior query semantics;
current revival checkpoint replay reproduces all eighteen runtime files and the
73-name ledger exactly. Historical restoration removes only the admitted 73 Stats,
six program occurrences, three revival queries and their new outer gap, and 350
intrinsic support-domain declarations before checking the original evidence.
The command reuses the existing typed release
assembler; it does not call test staging helpers, replay the old publication chain
or provide a second semantic checker.

The closed manifest requires registry, schema, rules, routing, mapping, roles,
normalization, rewards, item rules and item-source policy. Tree normalization and
provenance are optional. Metrics can be supplied alone. A support evaluation
group requires metrics plus stages, preparation, input bindings and receiving;
support outputs are optional only within that group. Named query sets retain the
order authored in the manifest. Unknown fields, duplicate declarations or source
paths, duplicate query names and incomplete groups are errors.

Those six evaluation components are authored semantic content, without runtime
dependency headers. The compiler constructs their exact bindings after DSL
lowering against the final checked recipe, using the existing package validators.
Runtime-header fields in these source files are errors; published runtime files
still require exact headers and reject stale bindings. Content inherited from a
release is extracted only by consuming its fully checked owner. Shared aggregate
work limits cover the source graph before copying, and final release assembly
retains all coverage checks. The tracked release still has no evaluation bundle;
construction support is not new mechanic coverage.

Program sources are declared with `rule-source "rules/example.rule"`; their
membership packets use `rule-packet "family/packet.kdl" names="family/names.kdl"`.
Every declared rule source must be used, and packet references must name declared
rule sources. Semantic names resolve exact schema addresses and grant no new
authority. A source file may serve multiple owners; each occurrence remains
separately checked and charged against expanded-work limits.

Named contribution fragments use
`contribution-queries "queries/resources.kdl" names="names/resources.kdl"`.
Every query names its Stat and exact producer authority; typed empty values and
ordered ranks are explicit. Fragments append without deduplicating members,
duplicate query IDs fail across fragments, and every partial gap is retained.
A complete fragment cannot close another fragment's partial coverage. These
calculation inventories are distinct from the named imported metric query sets.

The `rules` constituent is program-free and query-free JSON metadata: an empty
contribution inventory may retain its coverage, but captured query members are
rejected even when no named fragments are supplied. Schema, tables, coverage and
other constituents still use typed JSON DTOs, with `plain` transport by default or
explicit `encoding="gzip"`. Names, membership packets, Stat declarations,
contribution fragments and rule source are plain UTF-8. Broader declarations and
the indexed binary runtime package remain planned.
The library receives injected bytes; the CLI resolves source paths within the
manifest directory and bounds file reads. Separate aggregate limits cover encoded
and decoded bytes, source/component/packet counts, expanded rule work, query/group/
member/binding/gap counts and typed assembly limits. Final validators retain the
graph-work limits and all semantic checks. Source failures never fall back to
embedded programs or captured query members.
Gzip accepts exactly one complete member and rejects trailing bytes or additional
members. Compression changes transport size, not mechanics or decoded limits.

Optional `expected_input` asserts equality with the assembled release-input
digest. It is an exact reproduction check, not a coverage certificate. Both CLI
hosts call the same `poe-optimizer-host` publication implementation, which writes
and syncs staged files before a no-clobber directory rename on Windows/Linux.
Host contains no game, Import or Authoring dependencies. The native CLI loads
published releases through the shared checked artifact reader. It no longer
calls publication assembly from metric or source-action loading.

The prerequisite validation split is implemented. Registry decoding returns an
immutable `OwnedIdRegistrySnapshot`; offline allocation resumes explicitly from
that checked history. `CheckedOwnedRecipe` retains shared schema, stored rules,
routing and the Engine-checked executable, including its stored-source identity.
Publication staging wraps it, while evaluator-artifact validation uses the checked
recipe directly. Metric release loading now shares these handles instead of
cloning packages and compiling the rules again. Standalone artifact loading uses
the same Engine compiler. `CheckedOwnedRelease` validates the complete component
graph and retains its immutable packages. The reader streams canonical values
against the supplied bytes, including the recipe manifest and release receipt;
rehashing a forged claim or noncanonical artifact does not make it valid. Ordered
query commitments and aggregate byte/work limits remain checked. The raw file
buffers are released after loading. Offline publication and revision commands
explicitly reconstruct or stage data when requested.

Runtime role DTOs, indexes and validators now live in `owned_skill_roles`; offline
catalog generation depends on them in one direction. Offline code still resides
in Import. The maintained allocator/publication core must move into Authoring;
obsolete transformation chains must be retired after preserving their useful tests.
This checkpoint removes runtime reassembly. Legacy Data/Engine feature removal and
the planned compact package remain separate migration work.

The immutable pre-migration IR corpus lives only in Authoring's
[`tests/fixtures/retained-rules.json.gz`](../crates/poe-optimizer-authoring/tests/fixtures/retained-rules.json.gz),
outside the production source inventory.
The historical receipt authenticates that reference. Corpus tests compare actual
tracked source graphs and exact owner/program placement with the archive; release
tests restore only archived program members into a copy of the new input and use
the existing assembler to check unchanged metadata, provenance and other artifacts.
These migration gates pass without introducing an alternate release reader.

The retained source release has no evaluation bundle. Added component producers
retain Partial owner and incoming-query coverage; they do not establish a complete
source census or final revival result.
The maintained release tests now join Original05's 55 selected allocations and
two implicit roots to their exact preserved source identities. They distinguish
45 reviewed intrinsic output families, six dependent/descendant-producing sources
and six unresolved sources. This reuses the existing import sidecar and full
family proofs; it is not a new runtime absence certificate. Neither support-source
classification nor an empty parser result proves that every numerical modifier
has been accounted for.
The same evidence layer joins the selected Class and existing shared Player
application. It checks the complete 148 literal initialization outputs, fixed
attribute loop and Class/unarmed source records against the watched channels.
Initialization conditions and Level context can affect later calculations, so
they and foreign modifier stores remain separate obligations. These checks
preserve source identity; they neither add a second importer nor complete the
Actor's numerical coverage.

Intrinsic item-template support discovery has a separate boundary from rolled
or socketed modifiers. The implemented source-backed tranche covers four complete base
records and their item constructors; the existing Firebolt supplier remains an
exact Modifier grant. A template's intrinsic domain does not certify every
equipped copy, its modifiers, or its numerical outputs. The maintained source
proofs and ordinary release checker validate the four exact declarations; the
remaining item sources and numerical coverage stay unresolved.

One next limitation is documented in the [Skill-to-Actor contribution proposal](owned-skill-actor-contribution-proposal.md):
direct Skill effects can name Player, while checked Skill-origin queries currently
admit only self recipients. Repeated Skill contributions also need an explicit
arithmetic policy. The proposed authority and exact-sum extensions await review;
the current runtime and its completeness gates remain unchanged.

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
