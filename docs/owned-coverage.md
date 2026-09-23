# ADR: Scoped numerical coverage for owned builds

**Status:** Proposed; diagnostics retention is implemented separately and changes no numerical gate.
**Date:** 2026-09-23
**Deciders:** Project owner for the numerical contract; implementation review for the proof model.

## Context

The end-state build contract requires all dependencies of requested outputs to have supported
semantics. Current DraftSession finalization already excludes unrelated stash records and
unselected presets. It deliberately requires complete selected authored rows, including disabled
and off-loadout rows. The Engine then applies one numerical completeness flag to the entire
compiled plan. A missing applicable rule or receiver can therefore withhold a metric that may
ultimately be independent of it. Unknown catalog rows that are never referenced are different:
they do not automatically block numerical coverage.

A dependency walk over implemented programs cannot prove that omitted mechanics have no effect.
An unknown modifier may add a contribution, change activation, grant an actor or skill, redirect
an action's source, or transform another modifier. Treating missing implementations as absent
edges would produce confidently incorrect optimization results. The current broad gate remains
correct wherever the possible reach of missing mechanics is unknown.

The five imported example drafts also lack complete selected semantic inputs. Refining Engine
coverage alone cannot make them evaluable, and this proposal does not bypass that earlier gate.

## Proposed decision

Keep complete OwnedEvaluationRequest as the evaluation entry point. Replace plan-wide numerical
closure only where compilation can establish a private occurrence- and channel-specific coverage
proof. Keep broad blocking whenever the possible effect domain is unknown. A proof is bound to
the request, definitions, rules, routing, metric mapping and coverage contract version; it is
compiled once, shared immutably by workers and invalidated by affected candidate edits.

Diagnostics and finite game-data contracts remain independent of PoB, UI state and source text.
PoB is an optional offline comparison source, not a runtime resolver for missing facts.

### Required proof facts

- Identify the exact entity/provider occurrence and value, contribution or transform channel.
  Repeated gems, item uses, actors and loadouts must not collapse into a definition ID.
- Separate direct producer uniqueness, contributor membership, reduction identity, semantic
  transform order, activation, provider/grant discovery and source routing obligations.
- Describe what an unimplemented or partially implemented semantic family may affect with
  reviewed finite target-domain bounds, or explicitly mark its reach unknown. Implemented
  write sets alone are not such an upper bound. Bounds are owned definition/rule data, not
  source field names, fixture allowlists or runtime guesses.
- Include indirect effects on target existence, conditional inputs, skill/actor grants and
  source selection. A potential grant cannot disappear merely because its body is missing.
- Validate declared bounds against all implemented programs they cover, including transitive
  grants and routes. An implementation escaping its declared domain invalidates the package.
  Acquisition/review must separately justify that bounds cover the omitted semantics too.
- Keep complete-empty membership distinct from unknown membership. A reduction identity is
  usable only after membership is proven closed; a conditional branch does not prove this.
- Preserve every ordered requested metric row. Proven metrics may return complete numbers;
  affected metrics remain explicitly unavailable. Search must never silently omit an objective
  or substitute zero for an unavailable value. Legality and oracle agreement remain separate.

This is a semantic contract, not a proposed wire schema. Choose a bounded representation after
reviewing cross-owner grants, global conditional effects, magnitude transforms and item-local
channels against real examples. Avoid adding a general theorem prover or another interpreter.

## Options considered

| Option | Correctness and cost | Consequences |
| --- | --- | --- |
| Retain whole-plan closure | Simple and conservative; no exclusion proofs required. | Unrelated missing applicable mechanics block otherwise computable outputs. Useful baseline and fallback. |
| Prove scope using reviewed effect-domain bounds | More compiler/data validation work; conservative for unknown reach. | Supports independently complete metrics and precise blockers without requiring every applicable mechanic first. Recommended end state. |
| Walk only the known program graph | Cheap to implement, but missing edges are unconstrained. | Unsound: omitted effects become invisible. Rejected. |
| Evaluate unresolved authored drafts | Requires a separate partial-request and input-dependency contract. | Much wider change, including selected disabled/off-loadout handling. Deferred; not authorized by this proposal. |

## Consequences

Coverage becomes a compiled property rather than a boolean inferred from the absence of known
errors. The result contract becomes more useful for both optimization and interactive exploration,
but data authors must make reviewable closure or upper-bound claims. Incorrect bounds are as
serious as incorrect formulas. Unknown reach retains present behavior and should remain common
until evidence supports refinement.

The runtime should consume compact channel proof flags/indices alongside existing dependency
indices. Discovery, domain expansion and proof validation happen during preparation with bounded
work and memory. Avoid cloning diagnostic trees for every candidate evaluation. A future WASM
host uses the same native model and owns its presentation separately.

## Action items and acceptance

1. [x] Preserve exact Core binding diagnostics in the immutable effect plan and CLI reports;
   report all-draft versus explicitly selected draft issues separately. No numerical change.
2. [ ] Review this proposed numerical contract with the owner before changing coverage gates.
3. [ ] Specify bounded owned effect-domain declarations and their authority/validation rules.
   Include unknown reach and versioned identities; do not infer bounds from missing code.
4. [ ] Implement per-channel proof compilation for complete requests. Preserve existing behavior
   for unknown reach and preserve all requested rows and unavailable causes.
5. [ ] Validate unrelated known gaps, unknown-reach contamination, missing direct producers,
   complete-empty versus unknown contributors, activation/grants, cross-owner source routing,
   ordered transforms, reused item/gem definitions and A-to-B-to-A candidate edits. Compare
   fully covered requests with the existing whole-plan path and optional independent oracle.
6. [ ] Measure preparation cost, memory and parallel evaluation throughput on realistic builds;
   prove coverage diagnostics do not introduce per-evaluation binding or full-report cloning.

No action above establishes full build parity or retires the legacy backend. Complete imported
inputs, mechanics breadth, legal candidates and reference agreement retain their existing gates.
