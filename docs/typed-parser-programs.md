# Typed parser programs

Status: initial implementation contract for the accepted [parser language decision](conditional-parser-operations-proposal.md).
The owner chose the broader typed rule language on 2026-09-10. This document specifies
end-state responsibilities and the first delivery boundary. The [implementation record](implementation.md)
tracks completed work. G1 now provides the standalone program schema, structural/binding
verifier and immutable engine plans. No typed-program runtime or new callback admission
exists yet; source lowering and package dispatch still require G2-G4.

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

| Layer | Responsibility |
| --- | --- |
| Data crate | Serializable program/operation types, immutable definitions, schemas, source provenance and structural validation |
| PoB tooling | Authenticate source bodies/bindings; lower supported source syntax to programs; export definitions and original-source observations |
| Engine | Bind programs to their owning catalog, compile immutable plans, execute with per-invocation locals/heap/budgets, produce structured parser values |
| Import/native consumers | Retain existing parser call conventions, copy boundaries, diagnostics and separate numerical admission |

The initial serialized shape is an additive `ParserProgramCatalog` containing its own
schema version, program definitions and an explicit callback-to-program map. Each program
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
| Locals and assignment | Lexical slots, declaration scope and source order. Declared but uninitialized Lua locals start at Nil; missing fixed arguments also become Nil. Reassignment preserves value/table identity. |
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
hidden insertion counter. Prove the dense invariant before using the dense shortcut. Mixed constructors with
explicit numeric keys can also interact with Lua VM list-field flushing. Preserve the
source overwrite behavior or explicitly defer those forms until proved; source order
alone does not justify a naive sequence of immediate table writes.

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
Lower source method calls with their receiver lookup before invocation or argument
coercion. For example, `name:gsub(...)` must not become an unconditional host
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
Plans are immutable and shareable. Runtime capability checks, invocation tables and error
order remain G2 responsibilities; no public parser callback uses these plans yet.

The standalone wire model has schema1. The existing schema26/parser6 package and its
legacy recipes are unchanged. Program serialization, source export authentication, content
fingerprints and dispatch integration enter the package together in G4. Implementation
fingerprints already include the new Rust modules. Scope/ownership metadata must not be
mistaken for a static proof that every dynamic table write is permissible: reached writes
must check the invocation-owned heap, including aliases and values returned by helpers.

## Delivery sequence

1. **G1 — Schema and compiler:** versioned generic program model, source map, structural/
   effect verifier and immutable compiled plans; existing expressions remain unchanged.
2. **G2 — Execution:** locals, control flow, value/result packs, table heap and primitive
   binding with meaningful semantic and isolation tests. No callback admission by scaffold.
3. **G3 — Complete source lowering:** lower and prove the two original algorithm bodies
   through generic instructions, including raw-call and constructor/copy boundaries.
4. **G4 — Package and parser integration:** explicit dispatch, authenticated export,
   version/fingerprint updates, two reproducible exports, public/corpus regression and
   native-only/WASM dependency checks. Report exactly which callbacks gained capability.
5. **G5 — Subsequent real dependencies:** extend iteration/effect/callable semantics where
   reached callbacks require them. Preserve prior real programs and avoid named handlers.

These stages execute the accepted architecture; implementation details need no separate
owner approval. A change to the chosen direction or product behavior would still be
brought back for discussion.
