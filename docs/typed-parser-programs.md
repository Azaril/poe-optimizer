# Typed parser programs

Status: initial implementation contract for the accepted [parser language decision](conditional-parser-operations-proposal.md).
The owner chose the broader typed rule language on 2026-09-10. This document specifies
end-state responsibilities and the first delivery boundary. The [implementation record](implementation.md)
tracks completed work. G1 provides the standalone program schema, structural/binding
verifier and immutable engine plans. G2 adds a native invocation executor and raw graph
observation API. G3 adds authenticated whole-function lowering and independent raw
source comparisons for the first four functions. G4 packages the generated inventory and
explicit permissions, with native public dispatch, original public-copy parity and shared request
accounting. Four programs are admitted; 84 generated programs remain unadmitted.

## Purpose and boundary

Represent modifier-preparation algorithms as injected, versioned programs. Rust implements
the language primitives, validation, binding and execution machinery; configuration data
contains control flow, literals, lookup definitions and source-specific choices. Original
PoB remains the optional reference/extraction backend. Normal native execution requires no
Lua, process isolation or runtime Lua source interpretation.

Reuse the existing expression language and its proved primitives. Extend algorithm
structure through shared instructions instead of adding a native handler for each callback,
modifier line, skill or build. This is a parser/preparation language, with an execution seam
that can support later compilation. It does not prescribe the implementation style of all
combat calculations and does not replace the separate shared-build-model decision.

## Data, binding and runtime ownership

The [shared source-program contract](shared-source-programs.md) extends ownership and
per-build state beyond the parser while retaining these parser admission and copy rules.

| Layer | Responsibility |
| --- | --- |
| Data crate | Serializable program/operation types, immutable definitions, schemas, source provenance and structural validation |
| PoB tooling | Authenticate source bodies/bindings; lower supported source syntax to programs; export definitions and original-source observations |
| Engine | Bind programs to their owning catalog, compile immutable plans, execute with per-invocation locals/heap/budgets, produce structured parser values |
| Import/native consumers | Retain existing parser call conventions, copy boundaries, diagnostics and separate numerical admission |

The serialized `ParserProgramData` contains its own schema version, program definitions
and an explicit callback-to-program map. `ParserProgramCatalog` binds that data to its
immutable parser owner; `ParserProgramPayload` adds package-level permissions. Each program
has a parameter/result convention, local slots, structured body, dependency declarations
and complete source provenance. IDs belong to the exact catalog instance; equal numeric
IDs or matching labels in another catalog never establish valid ownership.

Keep all existing callback/table IDs and Pure factory recipes stable during the first
migration. Initially map only callbacks whose legacy disposition is Unsupported, with an
explicit dispatcher rule: use its validated program when present; otherwise preserve the
existing disposition. Reject overlapping executable Pure/program definitions instead of
silently selecting one. Legacy expressions remain supported and can later lower internally
to shared execution instructions without changing their serialized meaning.

A likely package migration is package27/parser7/program-schema1. Confirm the actual next
versions at implementation time. This is a proposed version assignment, not an exported
package. Source body, callback captures, global/intrinsic bindings, lookup construction and
program lowering semantics participate in identity/fingerprints. Do not copy the historical
native schema25 identity onto a new package or a reference run.

## Program model

Use closed Rust enums for operations and structured control flow. The serialized language
is not arbitrary executable text. A compiler resolves references and lowers blocks to a
compact immutable plan; the first executor interprets that plan in Rust. A future AOT or
other compiler must satisfy the same observable behavior and budget contract.

| Construct | Required semantics |
| --- | --- |
| Locals and assignment | Lexical slots, declaration scope and source order. Declared but uninitialized Lua locals start at Nil; missing fixed arguments also become Nil. Reassignment preserves value/table identity. Evaluate the complete RHS pack before storing local destinations right-to-left, including repeated destinations. |
| Conditional blocks | Ordered `if`/`elseif`/`else`; only the selected body runs. Truthiness rejects only Nil and false. |
| Short-circuit expressions | `and`/`or` preserve operand values and evaluation order; they are not eager Boolean reductions. |
| Equality and scalar operations | Explicit Lua-compatible operations over tagged values, reusing proved numeric/string primitives and preserving selected source errors. |
| Lookups | Dynamic keys against bound tables. Missing entries, including a Nil lookup against ordinary tables, remain Nil; later concatenation or calls can still fail. No sorting or invented ambiguity winner. |
| Tables | Ordered construction, lookup, dynamic-key set/delete and append through the invocation heap; preserve list index advancement and Nil holes. |
| Iteration | Bounded numeric/dense/pattern iteration with the original initialization, stopping and yield rules. Generic pattern iteration retains its result pack and source assignment adjustment. |
| Calls and return | Explicit targets, argument/result packs, early return and lexical control transfer. Preserve zero results, one Nil result and multiple results. |
| Break | Valid only inside its enclosing loop; unwinds the correct lexical scopes. |

These are typed instruction and value contracts. Static validation must not assume a
captured/input value has a source-required runtime type and then raise its error before the
source would. Dynamic values retain tags, and operation type checks happen when reached.
Use inferred types to optimize only where proof preserves behavior. An invalid structural
reference is a data error; a reached source type error is a source error; an unavailable
semantic capability is a separate preparation diagnostic.

The first source programs require branches, locals, lazy alternatives, dynamic lookup,
fresh tables, dense append/index/length, pattern iteration, concatenation, numeric conversion,
constructor calls and helper calls. Numeric/dense loops are part of the language direction;
implement and prove each execution form before any source lowering uses it. Additional loop
forms must preserve source stopping behavior while charging fuel; no silent trip-count cap.

## Values, identity and effects

The runtime value family includes Nil, Boolean, Number (including nonfinite values), raw
byte strings, table references and callable identities. UTF-8 is not a runtime Lua string
restriction. Avoid using Rust structural `PartialEq` as Lua equality: tables compare by
identity, NaN is unequal to itself and signed numeric zeros compare equal.

Distinguish immutable definition-table references from invocation-owned mutable table
handles. Locals and nested tables can share an owned handle; writes through one alias must
be visible through all aliases. Runtime keys retain their source domain: byte strings,
numbers, Booleans and supported reference identities. Canonicalize equal numeric keys
(including signed zero), without converting bytes to UTF-8 or formatting keys as strings.
Nil/NaN reads against ordinary tables miss; Nil/NaN writes produce source errors when
reached. Preserve identity for table/callable keys where admitted. A key domain not yet
implemented is an explicit missing capability. The existing public result model only
supports UTF-8 named keys and i64 indices: a bridge must preserve admitted keys or report
an explicit unsupported representation, never drop or stringify them.

The current public `Arc<ModifierTable>` representation is
an immutable result model, not the mutable language heap. Copy-on-write that detaches an
alias is not an implementation of Lua table assignment.

The first effect capability permits reads of authenticated captures/definitions and writes
to fresh invocation-owned tables. It does not permit mutation of shared catalog data.
The source targets named below satisfy this narrower effect contract; it is not a claim
that all parser callbacks do. Later source consumers needing argument/global mutation
require explicit effect/ownership support under the language model. They remain visible
as missing work rather than being cloned into an apparently pure computation.

Table length must use the applicable source semantics. For an append-built, proven dense
sequence, length is its dense count; arbitrary table length is not the number of stored
keys. Likewise, `ipairs` stops at the first Nil. Unproven `pairs` order, metatables and
dynamic callable behavior cannot be replaced by sorted iteration or a convenient default.
A table literal advances its implicit list index even for Nil values. A source append
operation instead uses the applicable current length; these operations cannot share a
hidden insertion counter. Prove the dense invariant before using the dense shortcut.

The runtime's table expression is an ordered IR construction, not a direct interpretation
of Lua table syntax. LuaJIT can hoist constant fields into a table template before dynamic
writes, including duplicate named keys as well as numeric/list keys. Source lowering must
encode template initialization and subsequent dynamic expressions in their actual order,
including error timing and alias observations. The G2 oracle pairs authored IR for these
cases; it does not establish a complete Lua-to-program lowerer. Defer any source form
whose overwrite/flush behavior has not been proved.

Preserve source copy boundaries. Raw program calls share handles as the source does;
constructor-specific copies and the public parser's recursive copy occur only at their
proved boundaries. Do not bridge calls by serializing or deep-copying every table. Adapt
existing constructor primitives to preserve heap identities or lower their required
behavior through the same language. Alias/copy parity is a gate for the chosen bridge.
Cycles, escaping references and recursion require bounded graph handling; no silent
flattening or accidental Rust stack overflow is acceptable.

## Call and intrinsic contract

A call target is either an authenticated primitive or a program bound to the original
callback/closure and its environment. A helper resolves its own captures, not those of its
caller. A function with the same body but different captures remains a distinct closure.
No program receives access to ambient process state or arbitrary host functions. Callable
identity is not complete closure introspection: preserve the existing opaque constructor
function-value observation boundary and defer cases that require an unproved captured
closure graph. A program mapping alone does not establish that broader parity.

Use result packs with explicit cardinality. Source assignment and argument-list adjustment
are explicit operations: fixed positions consume one value, while a permitted final call
can expand its results. A helper that reaches its end returns zero values; a caller can
adjust that to Nil without erasing the raw distinction. Unsupported vararg/tail forms are
rejected during source lowering until their semantics are implemented and proved.

The parser's existing Special/Prefix/ModTag conventions stay outside raw invocation. For
example, Special performs its own leading numeric conversion before calling the callback.
A program-to-helper call must not accidentally reapply those parser conventions.

Intrinsic identities and capabilities are versioned and source-bound. Reuse the existing
proved matcher, byte/string operations, conversion and constructor behavior. Pattern
syntax errors retain their source evaluation point, even if pattern preparation is cached.
Lower source method calls with receiver lookup before argument evaluation. Retain the
resolved method value, then evaluate arguments before checking whether that value can be
called; do not perform a second lookup after argument effects. For example, `name:gsub(...)` must not become an unconditional host
`string.gsub(name, ...)`: a numeric receiver can fail during indexing even where the
free function would coerce that number. Unsupported metatable/dynamic-method behavior
stays explicit.

A read-only value lookup and an identity-consuming lookup may have different ambiguity
requirements; preserve the relevant catalog construction rules.

## Validation, budgets and diagnostics

Validate schema versions, IDs, scopes, declaration/initialization rules, call bindings,
result contracts, effect permissions and structural sizes before publishing a compiled
plan. Analyze all admitted branches. An unsupported instruction cannot disappear because
a current fixture happens not to select it. Source-dependent errors remain lazy.

Bound nesting, instruction count, constants, locals, call depth, live heap values, table
entries, byte strings, output size and pattern work. Charge executed instructions and
loop/backedge work. Budget exhaustion returns a resource-bound failure with source context;
it never returns a truncated table or a partial successful modifier result. Allocation
checks happen before growth. Limits protect native and WASM hosts without OS processes.

Every instruction can map back to program/source location. Distinguish invalid data,
source error, missing/unimplemented capability and resource exhaustion. Diagnostic traces
are optional and bounded; they are not mandatory allocations in the normal execution path.
Plans and catalogs are immutable/shareable, while locals, heaps, call stacks and budgets
belong to each invocation or explicitly reset worker scratch.

## First complete source programs and proof

GemProperty at the pinned ModParser source normalizes an empty type, maps attribute
requirements, builds a requirement table, tests three lazy lookup alternatives, then uses
pattern iteration to choose a keyword or keyword list. A lookup hit forces the level
property while retaining the normalized type as keyword, including `"all"` after an
empty input. Program data must express these
choices; Rust must not dispatch on `GemProperty`, its source line or a fixture name.

grantedExtraSkill removes every occurrence of the source-defined substring `" skill"`
through an unanchored global substitution, performs a captured lookup, conditionally
converts a level and constructs an extra skill. A lookup miss skips numeric conversion and
returns zero values. Its caller/helper capture chain and first-result adjustment of gsub
are part of the proof, not assumptions inherited from the GemProperty path. Public grant
capability also requires the two original forwarding callbacks at pinned ModParser lines
3589–3590, including their Special argument packing; lowering the helper alone does not
admit those public lines.

For each program, authenticate the complete original function and relevant writers,
environment/captures and primitives. Compare native execution against the untouched source
body across all branches, error order, empty/missing inputs, numeric/nonfinite values,
raw bytes, aliases and injected alternate definitions. Validate raw function results
separately from parser-boundary adjustment and public-copy behavior. Unselected source
error paths must remain unexecuted. Preserve the original LuaJIT interpreter/compiled-
execution evidence distinction: warmed claims require uncached original callback/helper
calls and completed, live trace evidence, not public-cache hits or aborted recordings.

The first matrix must include distinct exact/load/minion lookup hits, falsey/truthy lookup
values, arbitrary requirement/property inputs, zero/one/multiple yielded words, dynamic
keys, method-receiver errors, constructor shapes, helper lookup misses, and source failures on otherwise
unselected paths. Shared primitive tests alone cannot establish full-program parity.

After full-program parity, validate public parsing and current corpus reports, preserving
all original inputs and fixed numeric expectations. Test changed injected program constants
and lookup definitions, concurrent catalogs with overlapping IDs, and reused execution state
against fresh state. Measure compile/preparation cost, execution and worker memory on the
real programs. This establishes parser capability only: full native build coverage remains
subject to the independent R1-R5 integration gates.

## Delivered G1 boundary

The data crate's `ParserProgramCatalog` retains its owning `ModifierParserCatalog`; it
accepts authored typed data or bounded JSON through `from_bytes`. The iterative verifier
checks every structural branch, lexical declarations and mutable parameter slots, source
locations/provenance linkage, callback mappings, helper/intrinsic bindings, result packs
and aggregate limits. Unknown instructions/fields and conflicting executable mappings
are rejected. These checks establish structural validity, not original-source equivalence.

The engine's `CompiledParserPrograms` lowers structured statements to source-mapped
instructions with explicit branches, loop state and return/fallthrough. Calls are prebound
to the retained library's program indices, existing recipes or declared intrinsics. It
preserves bounded expression trees, including lazy operators, without evaluating values.
Plans are immutable and shareable. The native executor below supplies runtime capability
checks, invocation tables and error ordering. G4 integrates explicitly admitted programs
with the ordinary public parser constructor.

The standalone program wire model has schema1. Package27/parser7 adds the required program
payload and permissions; legacy recipes retain their previous definitions and dispositions.
Serialization, source export authentication, content fingerprints and public dispatch
participate in the same package identity. Implementation fingerprints include the data,
compiler, executor, public adapter and extraction-policy sources. Scope/ownership metadata must not be
mistaken for a static proof that every dynamic table write is permissible: reached writes
must check the invocation-owned heap, including aliases and values returned by helpers.

## Native execution boundary

`CompiledParserPrograms::execute(callback, &ProgramValueGraph, ProgramLimits)` runs one
invocation with fresh mutable state. The returned `ProgramOutput` retains the owning
parser catalog, raw return pack and reachable graph, instruction/primitive-work counters
and allocation-accounting totals. Catalogs/plans can be shared by concurrent workers;
invocation heaps, locals, loop controls and budgets cannot leak across calls or catalogs.
Normal dependencies contain no Lua or process host. LuaJIT appears only in source tests.

The graph distinguishes zero results, explicit Nil slots, raw bytes, IEEE numbers, table
handles and opaque callback identities. Table aliases, cycles and identity-valued keys
survive calls and export. Input and catalog tables are read-only; a reached write to one
reports an unsupported effect, without cloning it into an apparently writable replacement.
Fresh tables remain writable through aliases passed to helpers. Helpers select their own
captures from the retained owner. Graph import/export are iterative and bounded.

Implemented execution includes locals and simultaneous assignment, lazy value-returning
operators, branches/breaks, numeric loops with hidden control state, live `ipairs`, pattern
iteration, fixed and expanded result packs, table access/construction, basic IEEE arithmetic,
byte comparisons/concatenation and source-bound primitive calls. Numeric-loop direction
preserves signed-zero and signed-NaN step behavior observed in interpreted LuaJIT. String
method resolution and call-time failures have distinct evaluation points. String `gsub`,
default/explicit-base10 `tonumber`, append-only `table.insert` and raw `createMod` have identity-preserving
bridges; passing arguments to raw helpers does not apply Special/Prefix/ModTag conversion.

Failures distinguish invalid input, source error, resource exhaustion and unsupported
capability. Source-mapped instruction/expression errors retain the innermost callback and
location. Limits cover execution work, call/nesting depth, result-pack count, logical value
storage, tables, byte payloads and pattern work. Primitive numeric scans and byte comparisons
share the pattern work counter, so `pattern_steps` is broader than pattern matching alone.
Allocation totals are cumulative charged units, including transient packs and export, not
measured peak resident memory or allocator overhead. A bounded failure publishes no partial
successful graph. Success or failure discards invocation state.

Capability gaps remain explicit: the raw legacy-factory bridge, modulo/power arithmetic,
nondecimal `tonumber` bases, unproved host-dependent integer conversions, positional table
insertion, dynamic replacement functions/tables, escaping iterator closures, general
metatable/callable behavior, borrowed-table mutation and arbitrary sparse-table length.
Extend these where complete source programs require them; compilation alone does not
certify every dynamically reachable operation. Complete source admission must inventory
all branches and required capabilities, including paths absent from current fixtures.

The initial tests compare authored IR against interpreted LuaJIT control functions and
original `createMod`, plus separate runtime isolation/resource tests. They are not complete
GemProperty/grantedExtraSkill translations, automatic source lowering, warmed-JIT evidence,
whole-build parity or performance measurements. G3/G4 retain those distinct gates.

## Whole-source extraction boundary

The optional PoB adapter exposes `parser_programs::extract_pinned` and
`extract_from_sources`. Both authenticate the pinned source inventory before executing
source construction. The latter accepts only normalized, manifested source bytes. Fresh
original construction must reproduce the caller-owned parser definition projection
byte-for-byte, including signed zero, captures, definitions and legacy dispositions. The
projection excludes only the new program payload, avoiding self-referential identity.
Program extraction retains that exact owner. Normal package extraction generates the full
inventory, applies the explicit admission policy and validates the resulting package.

Capture original primitive/library identities before construction and verify them afterward,
including string method lookup, `gmatch`, `gsub`, `table.insert`, `ipairs`, `tonumber` and the
original `createMod` closure. Complete source spans, observed capture identities and the
lowerer implementation participate in provenance. This adapter executes Lua offline;
consuming and executing the resulting typed catalog remains native and independent of PoB.

The generic lowerer translates complete function bodies, including lexical scopes,
initializers, branches, loops, result expansion, helper calls and permitted table writes.
Unsupported syntax anywhere in a body rejects the entire function, even in an unselected
branch. A helper call is retained only when the captured helper's own complete program is
available. Limits bound source bytes/tokens, expressions, blocks, locals and total program
size. No callback, skill or build name selects a runtime algorithm.

Source constructors currently require unique static string fields and disjoint implicit
list indices. Computed/numeric/duplicate keys are rejected because original LuaJIT
constant-template ordering needs further proof. Separate dynamic table writes remain
expressible. Mixed or multiple indexed assignments, scalar vararg positions, missing
operators, general iterators/callables, nested functions and other unsupported effects
remain explicit failures. The lowering report inventories every non-legacy callback as a
program or an unsupported reason; structural acceptance does not establish behavioral parity.

The first source matrix covers complete GemProperty, grantedExtraSkill and both grant
forwarding callbacks, including helper-owned captures and the original constructor. The
matrix compares raw return graphs before public Special packing/copying. It includes all
original gem lookup entries, alternate injected definitions, errors, byte/nonfinite/alias
cases and live warmed controls. Opaque function identity and definitions outside the
validated data model have separate controls; they are not promoted to graph parity claims.

Package admission must be explicit and backed by the applicable complete-function and
public-boundary evidence. It must not dispatch every program merely because extraction
emitted structurally valid instructions. Keep unproved generated programs out of public
execution while retaining their inventory for follow-up. G4 owns public result adjustment,
copy/cache behavior, authenticated package export, fresh corpus regressions and measured
preparation/execution cost. The implementation record states the current proved/unproved
counts and precise validation scope.

## Packaged public execution boundary

`ModifierParserData.programs` is a required `ParserProgramPayload`: a complete generated
`ParserProgramData` inventory plus explicit admissions. An empty permission map executes
none of its programs. The wire versions are package27/parser7/program1. Older packages
fail version validation rather than acquiring guessed permissions. The normal
`CompiledModifierParser::new` constructor validates and compiles the caller's payload;
no build, skill or callback name selects a Rust implementation.

Each admission binds exact parser definition bytes, serialized IR and source provenance,
with a Special or Helper role and an evidence reference. Special permission requires a
final Special-dictionary entry; every statically captured program dependency needs its
own permission. Helpers cannot become public entries by acquiring a helper permission.
Validate all generated programs, including unselected branches and unadmitted programs.
The bound catalog retains immutable ownership without an Arc cycle. `GameDataSnapshot`
exposes this same validated seam to native consumers.

An admission is an authored capability/evidence claim under the package's existing trust
policy. Its hashes detect changed definitions, source or instructions; they are not a
cryptographic certificate of behavioral parity. A custom caller can deliberately author
and rebind a different program, with the resulting package identified as custom/unreviewed.
Reviewed exports use a separate source/IR policy file. The generic lowerer still inventories
every function; the policy does not contain production callback-ID dispatch handlers.

Public Special invocation applies its existing leading numeric-capture conversion once
at the call site, retains the remaining raw captures and executes with a fresh invocation
heap. The wrapper adjusts the raw return pack to its first two results, then uses a
bounded graph adapter and the normal public deep-copy path. Native parsing retains no
result cache; its returned copies reproduce the original parser's cache-boundary behavior.
Aliases survive the raw program boundary. Trailing results are ignored by the wrapper.
Unsupported public shapes, including false/scalar extra results, non-UTF-8 byte keys and
noninteger keys, remain explicit deferred results rather than silently losing data. Cyclic
public copies and exceeded traversal/depth bounds are resource failures, which the importer
retains as ResourceError. Valid UTF-8 string keys are supported.

A parse request retains three separate, failure-inclusive counters across both ordering
passes and retries: ProgramRequestAccounting covers native execution, graph import and
freeze; OutputBudget covers caller captures, raw-to-public adaptation and public copying;
MatchBudget covers scans, primitive work and adapter traversal. Public adaptation does not
debit the program counter itself. Charge input/result slots and byte payloads before
allocation; retain charges after failed calls. Instruction, allocation and nesting limits
remain separate bounds. Import-owned diagnostics preserve source error, resource error
and unavailable-capability distinctions with callback and source location.

G4 admits three public Special entries and their one helper after complete original-body
raw and public-wrapper validation. The other 84 generated programs remain unadmitted.
The evidence includes changed injected definitions/programs, cache isolation, alias copying,
result adjustment, source failures, live warmed execution, identical fresh package exports
and supplied-corpus regressions. Exact counts and identities belong in the implementation
record. A separate native parser benchmark shares immutable plans among Rayon workers;
its timings and memory are parser measurements, not whole-build or optimizer throughput.
Full build breadth still depends on the separate R1-R5 integration gates.

## Delivery sequence

1. **G1 — Schema and compiler:** versioned generic program model, source map, structural/
   effect verifier and immutable compiled plans; existing expressions remain unchanged.
2. **G2 — Execution:** locals, control flow, value/result packs, table heap and primitive
   binding with meaningful semantic and isolation tests. No callback admission by scaffold.
3. **G3 — Complete source lowering:** lower and prove the two original algorithm bodies
   through generic instructions, including raw calls and the original constructor.
   Retain public-copy/dispatch proof as the explicit G4 integration gate.
4. **G4 — Package and parser integration:** explicit dispatch, authenticated export,
   version/fingerprint updates, explicit evidence-backed admission, public-copy/cache proof,
   two reproducible exports, public/corpus regression and
   native-only/WASM dependency checks. Report exactly which callbacks gained capability.
5. **G5 — Subsequent real dependencies:** extend iteration/effect/callable semantics where
   reached callbacks require them. Preserve prior real programs and avoid named handlers.

These stages execute the accepted architecture; implementation details need no separate
owner approval. A change to the chosen direction or product behavior would still be
brought back for discussion.
