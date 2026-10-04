# ADR: Shared source properties during support preparation

**Status:** Accepted on 2026-10-03: reuse existing skill occurrences. Detailed wire contracts, source evidence and implementation remain open.
**Date:** 2026-10-03.
**Decider:** Project owner.

## Context

The accepted [occurrence-input model](owned-skill-occurrence-input-proposal.md)
keeps semantic raw inputs on exact skill occurrences with explicit producer
authority. The accepted [readiness model](owned-preparation-readiness-proposal.md)
separates preparation inputs from final execution inputs on one occurrence graph.
Neither decision establishes how several supplied effects share supported level
and quality properties exactly once per source.

This is a real input blocker. Ice Nova's intrinsic program now reads required
final level `32d6`, but no real producer supplies it. Native arithmetic and
declared-child projection already exist. Assigning its saved raw level directly
would skip external and supported properties. The two stat sets are alternatives
of one effect, so properties must not accumulate once per requested Action.

In the pinned source, `CalcActiveSkill.lua:236-262` collects supports associated
with the exact source object, merges their modifiers and counts non-hidden
supports. It caches the resulting property list by that object. Each eligible
effect applies those properties before final level validation. Source-level
aggregation and per-effect application are different operations. The first
cache reader also supplies an actor/query context; native parallel scheduling
must never choose that context accidentally.

The native graph already has exact authored and generated Skill targets, and
`ConcreteEntity::Skill` can hold occurrence-specific numeric channels. However,
current support receiving endpoints and supported preparation writes are limited
to Actor/Action. `AssignedSkill` resolving an address does not authorize a
source-wide reduction or establish effect membership.

## Accepted decision

Add a versioned, data-defined source-preparation relation over existing skill
occurrences. Use an exact existing Skill target as the input/property owner;
do not add a second build model or copy its raw input slots into another entity.
This is new checked relationship and invocation authority, not a new evaluator
or arithmetic interpreter.

The proposed contract has these requirements:

1. Definitions declare the finite eligible effect endpoints through exact grant
   paths, their raw-input binding and incoming actor/query context. Membership
   must be Complete before aggregation. A shared ancestor or definition name
   does not establish membership. Minion children and support-supplied effects
   require their own declared semantics; descent alone does not admit them.
   The relation's identity includes its declaration and exact native Skill target.
   Multiple SkillUses can currently reference one backing Gem; require checked
   correspondence to the intended source instances before grouping or separating
   them. Neither the backing Gem ID alone nor an arbitrary first SkillUse is a
   valid grouping rule. Ambiguous aliases remain an explicit validation issue.
2. Ordinary per-effect support admission runs first. The relation then collects
   admitted exact support occurrences for that source, preserving provenance and
   the existing ordered-selection contract. Deduplication follows source-instance
   identity and verified retained-position behavior, never Gem definition ID.
   Non-hidden support count uses this same admitted census.
3. Authorized supported-property programs run once per admitted occurrence and
   source relation, contributing ordinary typed numeric channels to its exact
   Skill owner. Two independent uses of the same definition remain separate.
   These channels require complete incoming contributors, not an empty-list zero
   when producers or membership are unresolved.
   Actor/query-provided SupportedGemProperty records form a separate required
   contributor inventory: the source seeds its property list from the actor's
   ModDB before adding support modifiers. They must still be evaluated when no
   supports are admitted, with exact ownership, conditions and provenance.
4. Final assembly consumes unvalidated pre-support quantities and those complete
   property channels, then validates level last. Every eligible effect receives
   its own final inputs. Generated children use existing declared projection;
   Direct effective inputs remain Skill stat channels. This proposal does not
   authorize a Direct self-parameter writer or relabel raw inputs as final.
5. The relation binds a stable actor/query context or proves all eligible readers
   equivalent. An ambiguous context stays unsupported. PoB's cache is evidence
   about source behavior, not a worker-order-dependent native specification.
6. Preparation, aggregation and final assembly retain explicit dependencies and
   one shared work budget. Required-input, cycle, activation, contributor and
   complete-build gates stay intact. This proposal does not relax owner program
   completeness to make a partially converted build execute.

Runtime artifacts remain source independent. Import and offline acquisition
establish correspondence to external fields, types and objects. Compiled plans
use exact native identities and reusable worker-owned state. Invalidation must
include source identity, member effects, selected supports, raw/external inputs,
actor/query context and data-package identity.

## Options considered

| Option | Benefits | Costs and limits |
| --- | --- | --- |
| **A. Relation over existing Skill occurrences — accepted** | Reuses accepted raw-input storage, occurrence identities, channels and child projections. Adds only the missing membership and invocation authority. Fits physical and demonstrated Direct sources without another lifetime or duplicate input model. | Requires versioned relation/receiving/readiness validation. A source without a natural existing Skill owner must remain unsupported until its ownership is designed; choosing its first effect arbitrarily is invalid. |
| **B. First-class source-input entity** | Gives sources an independent identity and lifetime. Could support future evidence of inputs shared across independently owned skills or sources that have no natural Skill owner. | Adds persistence, binding, migration, raw-input ownership and relation machinery alongside the recently accepted Skill inputs. Current evidence does not establish a need for that extra lifetime. Avoiding duplicate storage would require a broader migration. |

Both options can run natively in parallel and support WASM. The trade-off is
ownership and migration cost, not Lua versus Rust or database choice. A finite
singleton with a proven empty property inventory could proceed under existing
contracts, but it would not answer the general ownership question or admit
nonzero supported-property changes during optimization.

Revisit A if a real source cannot be represented by an existing exact input owner,
or if independent skill lifetimes must share one configurable input source. Do
not stretch it by adopting an arbitrary first member as owner.

## Evidence and implementation gates

The current audit is `runs/owned-ice-nova-final-inputs-next-audit-01.md` at commit
`55fc91a`. Existing intrinsic, Djinn topology/admission and fractional-order proofs
remain useful. They do not prove Ice Nova's complete property census or cache
context equivalence.

- [x] Decide the ownership model: the owner selected reuse of existing Skill
  occurrences, with explicit source membership and aggregation permissions.
- [ ] Specify exact versioned wire contracts and structural, binding and execution
  validation before implementing authority.
- [ ] Observe original Ice external candidates, matches and rejected dispositions,
  exact admitted-source relationships, cache first reads/hits and final validation.
  Include genuine nonzero item and supported-property controls, separate physical
  copies and all relevant saved presets. Reuse existing numeric boundary proofs.
- [ ] Establish actor/context consistency and repeated-position semantics. A
  mismatch is an unsupported case or a revised design, not a silent parity exception.
- [ ] Implement the approved relation and source-target permissions in the existing
  graph; retain negative tests for incomplete membership, duplicate writers,
  cross-source leakage, cycles, wrong phase and missing contributors.
- [ ] Author real property producers and final assembly, including external
  modifier routing and the Amulet bonus-copy stream. Existing nonzero minion-level
  item producers do not establish the complete incoming inventory by themselves.
- [ ] Validate source components, unchanged originals, interacting mutations,
  reused/parallel execution and WASM. Do not claim a working original until its
  full request and contributor coverage pass.

Generated saved-input persistence and the pending
[usage-applicability decision](owned-generated-skill-usage-proposal.md) remain
separate. This proposal neither resolves those choices nor selects the canonical
PoB reference lifecycle.
