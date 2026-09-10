# ADR-Parser-01: Conditional modifier parsing

**Status:** Accepted — expand the typed rule language
**Date:** 2026-09-10
**Decider:** Project owner

## Context

The native modifier parser evaluates authenticated, bounded expressions over injected
definitions. Its existing string, Flag and one-argument numeric-conversion extensions
are implemented. Schema26/parser6 currently preserves 1,222 Pure recipes and 429
Unsupported dispositions. Those are preparation capabilities, not whole-build coverage.

Some remaining callbacks are algorithms rather than single-return expressions. The
complete GemProperty callback at pinned ModParser.lua:3526–3554 normalizes an empty skill
type, maps attribute-requirement names, performs ordered skill-name lookups, and splits
words to choose a single keyword or keyword list. A lookup hit deliberately selects the
level property even when another property was requested. Fifteen current corpus parser
stops reach this callback, but the implementation must cover its full behavior, not
special-case those builds.

The smaller grantedExtraSkill helper at lines 2193–2200 removes every occurrence of a
source-defined substring, looks up a skill and converts a level only on a hit. A miss
falls through with zero return values. Both use an already injected
961-row lookup whose own source construction sorts gem identifiers before assignment.
Its provenance must be proved independently of other lookup tables with different
construction or ambiguity rules. Trigger helpers mutate option tables, and extraSupport
also reads live global data; those require separate effect/state designs.

This decision concerns modifier preparation. The calculation backend remains native
Rust, data stays injectable, and PoB remains an explicit parity reference. Neither
option introduces Lua subprocesses into native preparation or candidate evaluation.

## Decision

The project owner selected **"Expand the typed rule language now"** on 2026-09-10.
This supersedes the earlier recommendation to implement complex callbacks as focused
Rust algorithm families. Extend the existing expression approach with a versioned typed
program representation for branches, bounded iteration, lexical locals, intermediate
values, table operations and source-bound calls.

Store algorithm structure and game facts in injected definitions. A native Rust compiler
validates and binds the program; an initial bounded Rust interpreter executes its compiled
form. Keep that execution boundary replaceable by a later compiler without changing the
serialized program or the calculation backend interface. Compilation here initially means
validation and lowering to an immutable execution plan, not machine-code generation.
No new `GemProperty` or `grantedExtraSkill` runtime opcode is introduced: both bodies must
lower through shared instructions and existing proved primitives. Grant-line admission
also requires their original forwarding callbacks and parser call conventions.

The [typed parser-program contract](typed-parser-programs.md) specifies the ownership,
call, effect, source-lowering and migration boundaries. The first complete source targets
are GemProperty and grantedExtraSkill, exercising different branch, lookup, iteration and
return-cardinality behavior. Broader language features are added under the same semantic
contract as reached source consumers require them; unsupported behavior is explicit.

This decision authorizes the language design and implementation work. It does not approve
the separate B3 shared build-model migration, which still awaits its own answer. Real-build
R1-R5 gates remain the integration priority; successful parser programs do not count as
complete native builds.

## Options considered

| Dimension | Focused Rust operations (earlier recommendation) | Extend the expression model (selected) |
|---|---|---|
| Representation | Explicit implementations for complete algorithm families | Add typed branch, loop, lookup and local-value instructions |
| Initial complexity | One operation and its injected definition at a time | Define and validate a larger executable data language first |
| Reuse | Share tested byte, number, lookup and constructor primitives | Reuse control-flow instructions across more source bodies |
| Execution | Direct Rust control flow; compile immutable definitions once | Compile immutable plans once; initially interpret them in Rust, with a later compilation seam |
| Update burden | Review each changed source algorithm and its data extraction | Maintain lowering plus all new instruction semantics |
| Main risk | Too many bespoke operations or literals creeping into code | Growing an incomplete Lua interpreter and losing error/alias fidelity |

## Trade-offs and consequences

The selected approach makes recurring control flow reusable and allows more future PoB
changes to be represented by data when the required instructions already exist. It also
provides a path toward more automated source lowering. Neither benefit removes the need
to prove translation fidelity or to add Rust primitives for newly encountered semantics.

We accept a larger initial implementation and debugging burden. The project must maintain
instruction semantics, binding validation, source maps, runtime budgets and an original
source oracle. Nil/false distinctions, lazy evaluation, return arity, number behavior,
identity, mutation and copy boundaries are language requirements, not incidental details.
A broad but incomplete Lua imitation is not sufficient evidence of PoB parity.

Focused Rust algorithms remain the considered alternative: easier direct debugging and
less initial infrastructure, but more manually maintained ports and a risk of accumulating
bespoke handlers. Existing proved native primitives remain reusable; the selected direction
places newly supported conditional algorithm structure in the shared rule representation.

Keep native execution portable, with immutable shared plans and invocation-local state.
Both approaches could satisfy Rust/WASM and parallel execution goals. No candidate-search
speedup is claimed from this parser decision without measurements of real preparation
workloads and cache reuse.

## Required proof before admission

1. Authenticate the complete source body, environment, lookup construction and writers.
2. Compare the whole operation over every branch, alias, empty/missing value and source
   error order; include raw bytes and custom injected definitions.
3. Exercise arbitrary requirement/property/type inputs and all lookup paths, independent
   of the supplied builds. Retain original quirks, including property replacement.
4. Prove source/public copy behavior, real warmed execution, bounded matching/loops and
   isolation between concurrent catalogs and requests.
5. Preserve existing recipes, data sections and raw corpus/reference values; report
   parser progress separately from item assembly and numerical capability.

## Action items

1. [x] Record the owner's choice of a broader typed rule language.
2. [x] Write the initial program, value/heap, binding, execution and validation contract.
3. [ ] Implement the versioned schema, structural verifier and immutable compiled plans.
4. [ ] Implement native control flow, invocation-local heap and explicit call/result packs.
5. [ ] Lower complete GemProperty and grantedExtraSkill source bodies through generic
   instructions; authenticate their dependencies and verify all branches against PoB.
6. [ ] Integrate callback dispatch, public parser/copy behavior and package export; prove
   preservation of existing recipes and independent injected-data behavior.
7. [ ] Extend effectful/table-iteration/callable capabilities as subsequent real-build
   dependencies require them, retaining the same parity and resource-bound requirements.

These are implementation checkpoints, not additional architecture approval requests.
Follow the current [implementation record](implementation.md) for execution status.
