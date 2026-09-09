# ADR-Parser-01: Conditional modifier parsing

**Status:** Proposed — awaiting user direction before implementation
**Date:** 2026-09-09
**Decider:** Project owner

## Context

The native modifier parser currently evaluates authenticated, bounded expressions over
injected definitions. The closed Flag extension preserves that design. The next
straightforward extension is one-argument `tonumber`, with 81 complete source candidates;
it can proceed independently of this proposal.

Some remaining callbacks are algorithms rather than single-return expressions. The
complete GemProperty callback at pinned ModParser.lua:3526–3554 normalizes an empty skill
type, maps attribute-requirement names, performs ordered skill-name lookups, and splits
words to choose a single keyword or keyword list. A lookup hit deliberately selects the
level property even when another property was requested. Fifteen current corpus parser
stops reach this callback, but the implementation must cover its full behavior, not
special-case those builds.

The smaller grantedExtraSkill helper at lines 2193–2200 strips a source-defined name suffix,
looks up a skill, converts a level and returns nil on a miss. Both use an already injected
961-row lookup whose own source construction sorts gem identifiers before assignment.
Its provenance must be proved independently of other lookup tables with different
construction or ambiguity rules. Trigger helpers mutate option tables, and extraSupport
also reads live global data; those require separate effect/state designs.

This decision concerns modifier preparation. The calculation backend remains native
Rust, data stays injectable, and PoB remains an explicit parity reference. Neither
option introduces Lua subprocesses into native preparation or candidate evaluation.

## Proposed decision

Implement focused Rust operations for complete, read-only parser algorithms, beginning
with GemProperty and subsequently reviewing grantedExtraSkill. Keep ordinary expression
recipes for simple composition. Do not add general branching or loops to the expression
language merely to translate these two functions.

Each operation would have:

- A versioned operation kind and complete source/function/captured-binding provenance.
- An injected definition holding lookup identity, patterns, branch labels, field names,
  literal values and required capabilities. Game data and build identities do not live
  in the Rust implementation.
- Exact ordered native value arguments, including nil, byte strings and non-finite
  values, with source errors raised only when the corresponding operation is reached.
- Immutable compiled definitions, per-request work/output budgets, and the existing
  structural result/copy boundary. No hidden fallback or shared mutable state.

The dispatch seam must distinguish a whole callback operation from a helper invoked
inside an expression. Its exact enum/API/schema shape would be designed after this
choice. Source recognition consumes the complete body and validates dependencies;
callback IDs or example-build names are not admission whitelists. An upstream body,
lookup construction or effect change requires renewed parity proof.

## Options considered

| Dimension | Focused Rust operations (recommended) | Extend the expression model |
|---|---|---|
| Representation | Explicit implementations for complete algorithm families | Add typed branch, loop, lookup and local-value instructions |
| Initial complexity | One operation and its injected definition at a time | Define and validate a larger executable data language first |
| Reuse | Share tested byte, number, lookup and constructor primitives | Reuse control-flow instructions across more source bodies |
| Execution | Direct Rust control flow; compile immutable definitions once | Interpretation or further compilation would need its own design |
| Update burden | Review each changed source algorithm and its data extraction | Maintain lowering plus all new instruction semantics |
| Main risk | Too many bespoke operations or literals creeping into code | Growing an incomplete Lua interpreter and losing error/alias fidelity |

## Trade-offs and consequences

Focused operations follow the existing native-calculation approach and make full-function
parity review manageable. They also require discipline: group behavior into coherent
algorithms, inject all selected game facts, and extract shared primitives when there is
actual reuse. This must not become one hard-coded handler per modifier line or build.

A richer expression model could reduce handwritten ports, but branches, loops and
lookups bring truthiness, evaluation order, temporary lifetimes, aliases and resource
limits into the data-language contract. Mutation and arbitrary callable values would
still require separate designs. Since parsing happens during preparation, neither
option should be justified by an unmeasured candidate-throughput claim.

For either choice, lookup outcomes must preserve the actual source table semantics.
When an operation observes only existence/truthiness, equivalent alternatives should
not cause an unnecessary failure. When it consumes an identity, unresolved alternatives
must not become an incidental winner. Source errors, missing results and unavailable
operations remain distinct.

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

1. [ ] Confirm focused Rust operations or the richer expression model with the user.
2. [ ] Write the selected operation/definition and dispatch contract before coding it.
3. [ ] Implement and validate the complete GemProperty algorithm under that contract.
4. [ ] Review grantedExtraSkill separately; keep mutable trigger/support helpers pending.

No implementation or B3 actor/action/candidate migration is authorized by this proposed
record. The independently reviewed one-argument numeric conversion phase remains
available while this choice is pending.
