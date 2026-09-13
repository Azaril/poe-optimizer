# ADR proposal: captured jewel effects at the item boundary

**Status:** Proposed investigation; no production representation selected.
**Date:** 2026-09-13
**Deciders:** Project owner, following the A2 comparison.

## Context

Two supplied originals now stop at the same captured jewel-effect family. Pinned
`ModParser.lua:7139–7156` captures a passive kind and modifier text, then returns a
function. The wrapper at 7377–7386 returns another function retaining it. The effect
parses its text only when invoked for an eligible node, mutates modifier provenance,
and appends the actual records to the output list. Parsing the captured text eagerly
would change execution timing, failures and possible shared state.

The current native parser reports this factory as unsupported. Its item adapter transports
finite metadata, and the owned item arena has no executable value variant. Adding factory
dispatch alone would leave both boundaries unresolved. Callback IDs in extracted data are
version-specific identities, not acceptable production dispatch constants.

These Time-Lost jewels also expose a nontrivial consumer: `CalcSetup.lua:167–185` invokes
radius effects on normal, attribute and notable stand-ins to populate a cache. The later
node consumer at 218–237 copies a matching cached list and stops at the first applicable
Time-Lost entry. Its invalidation path retains a local cache reference before replacing the
global cache, then continues using that earlier local reference (or nil) in the same call.
First-call and next-call behavior must therefore be compared separately. Scaling at 128–165
can rewrite newly emitted modifiers and their display text. A per-node callback interface alone does not capture this lifecycle.

Current native components provide two useful starting points:

- `ProgramSession` and `SessionValue` preserve executable source state and aliases within
  an owner-bound private session. Cloning a SessionValue preserves a handle, not an owned
  session copy; foreign-session handles are rejected. There is no general session clone/reset
  API, and snapshots reject live closures and class/proxy behavior. They do not yet cross the
  finite item adapter/arena or establish support for this nested factory/local-parser path.
- `AssembledItem` shares an immutable owned graph between consumers. A typed effect reference
  could fit that lifetime, but equivalence to source closures and deferred parsing is unproven.

These facts come from the pinned source and current Rust boundaries. This proposal contains
no measured speedup claim. The [R2af audit](implementation.md) retains exact inputs and source
hashes under `runs/r2af-rune-order-01/next-frontiers-audit.md`.

## Options

| Option | Representation and execution | Potential benefit | Cost or unresolved issue |
| --- | --- | --- | --- |
| Session-owned executable values | Keep captured functions and mutable dependencies in a private native source-program session; item graphs reference that owner. | Reuses source execution, identity and deferred-call semantics; may minimize new algorithm translations. | Couples item lifetime to a mutable session; requires explicit sharing, copying and worker isolation; extends interpreter and item interop. |
| Typed captured effects | Item artifacts retain validated effect definitions and captured data; a native domain executor receives explicit invocation context and output state. | Can keep immutable item data separate from worker-private execution, expose dependencies, and avoid carrying general session handles through item APIs. | Must model deferred parsing, state, aliases, scaling and cache consumers correctly; new families need a versioned operation/data contract and source adapters. |
| Bounded comparison first | Implement the same representative effect slice behind experimental adapters for both options. | Tests the structural choice before committing the production API; supplies an A2 example. | Adds temporary code and duplicate validation; requires an explicit decision and deletion plan. |

Both production options execute in Rust and keep injected game data. Neither requires PoB
subprocesses in native candidate evaluation. A typed effect is not permission to hardcode
item names, build identities, numerical answers or captured modifier text. A source-session
handle is not evidence that an item or radius effect is fully supported.

## Proposed decision

Compare the two representations on a bounded vertical slice before selecting the production
item/effect ownership API. Prefer the typed boundary only if it preserves actual downstream
behavior and lowers the combined implementation, update and execution cost. Retaining session
ownership remains valid. This does not select a universal DSL or replace all existing programs.

The initial slice is the captured passive-grant family shared by the two originals, plus
contrasting captured numeric and nested-modifier cases. The result boundary includes effect
construction, repeated invocation, output records, errors, relevant aliases and the stand-in
cache/scaling consumer. It must not stop at serializing a callback descriptor.

## Evidence and follow-through

1. Authenticate/select the complete family generically, including overlapping patterns and
   the wrapper. Establish actual compile/import/invoke support for each prototype before
   timing it. Preserve captures and deferred local-parser dispatch; existing callable APIs
   do not prove that this family is executable.
2. Run unchanged original functions against nil, eligible/ineligible node kinds, attribute
   nodes, malformed/unparsed text, repeated calls and nested modifier values. Retain ordered
   mutations and error prefixes. Identify every observable alias before changing ownership.
3. Exercise stand-in cache construction, scaling, item/tree/mode changes, invalidation and
   conflicting radius effects. Carry results into the actual selected-tree consumer; do not
   count import success as actor execution.
4. Compare private-session import/copy/drop against typed artifact preparation, invalidation
   and repeated effects. Include any required session-copy mechanism as new work rather than
   assuming a handle clone supplies it. Measure shared and worker-private memory, multiple worker counts,
   and total work; isolated effect timings do not establish full-build search throughput.
5. Apply the same injected value change and new conditional-family change to both. Count
   source acquisition, adapters, tests and migration code as well as runtime code.
6. Record the choice with the owner. Migrate the selected boundary, remove the rejected
   experimental path when its evidence is retained, and add full original-build comparisons
   as downstream calculation becomes available.

Keep unknown behavior explicit throughout. Existing unique-item ordering, equipment activation
and other bounded correctness work can continue independently while this choice is discussed.
This proposal feeds [A2/A3](rule-execution-model-investigation.md); it does not close those milestones.
