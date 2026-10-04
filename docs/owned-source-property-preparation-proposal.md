# ADR: Shared source properties during support preparation

**Status:** Accepted on 2026-10-03: reuse existing skill occurrences. The bounded source witness passes. Native contracts, the executable component, regression/preservation tests and portable checks pass on 2026-10-04. Real source-input fragments pass publication and native tests; complete contributor/owner coverage remains open.
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

An authored physical Skill target can be backed by a Gem definition; a Direct
target is backed by a Skill definition. The relation must preserve this existing
distinction. Its source owner is not necessarily a generated primary effect.

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
   Producer context and destination are distinct: a modifier or support program
   retains its checked source reads. Explicit relation authority binds its
   numeric destination to the source's Skill owner. Rebinding every producer's
   `Current` context to that Skill would lose or broaden raw-input authority.
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

The admitted support census must retain position identity until source evidence
establishes the reduction policy. The original collection loop visits every
retained position; two positions referencing one support object must not silently
collapse to one contribution. Source-wide aggregation happens once per relation,
but its input census can still contain repeated positions. Count and properties
must use the same verified multiplicity.

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

The original audit is `runs/owned-ice-nova-final-inputs-next-audit-01.md` at commit
`55fc91a`. The new [complete-source witness](owned-minion-spell-input-evidence.md#shared-source-property-collection-and-final-input-ordering)
passes twenty cases, three lifecycle stages and both JIT modes. It proves the
finite Ice external/property census, retained-position multiplicity, independent
copies and manual Djinn shared-cache context. Existing intrinsic, admission and
fractional-order proofs remain useful. Broader aliases, other actors and complete
native contributor inventories remain separate obligations.

- [x] Decide the ownership model: the owner selected reuse of existing Skill
  occurrences, with explicit source membership and aggregation permissions.
- [x] Specify exact versioned wire contracts and structural, binding and execution
  validation before implementing authority.
- [x] Observe original Ice external candidates, matches and rejected dispositions,
  exact admitted-source relationships, cache first reads/hits and final validation.
  Include genuine nonzero item and supported-property controls, separate physical
  copies and all relevant saved presets. Reuse existing numeric boundary proofs.
- [x] Establish actor/context consistency and repeated-position semantics in the
  bounded physical Ice/Twister and manual Djinn cases. Other contexts and aliases
  remain unsupported until proven; this is not universal context equivalence.
- [x] Implement the approved relation and source-target permissions in the existing
  graph; retain negative tests for incomplete membership, duplicate writers,
  cross-source leakage, cycles, wrong phase and missing contributors.
- [ ] Author real property producers and final assembly, including external
  modifier routing and the Amulet bonus-copy stream. Existing nonzero minion-level
  item producers do not establish the complete incoming inventory by themselves.
- [x] Validate the bounded native relation component, unchanged originals,
  interacting component inputs, reused/parallel execution and WASM. Real property
  authoring above remains open; full request and contributor coverage are still
  required before claiming a working original.

Generated saved-input persistence and the pending
[usage-applicability decision](owned-generated-skill-usage-proposal.md) remain
separate. This proposal neither resolves those choices nor selects the canonical
PoB reference lifecycle.

## Next implementation boundary

### Implemented contract

The receiving wire DTO adds an optional, non-null `source_properties` object;
V3 requires it and V1/V2 require its omission. Its relations name an authored
Gem-backed or Direct Skill owner, exact eligible
effect endpoints, a Player/scenario context, numeric channels, computed owner
inputs and complete external/support/assembly program inventories. The first
alias policy rejects shared backing Gems. Complete empty inventories are valid;
Partial inventories are rejected, including owners with no listed programs.
Older receiving versions retain their serialized representation when the
extension is omitted.

Operations V18 introduces the relation-only `PropertyOwner` scope. Producer
`Current`, Gem, modifier and parameter reads retain their existing meaning.
Stages V3 classifies external properties, supported properties and final assembly
separately. A private native census follows admission and precedes those property
programs; it cannot be authored as an ordinary count writer. An active empty
census produces zero, proven inactivity produces Inactive, and missing evidence
does neither. The operations default remains V14.

Cold compilation discovers owners from the selected build even without support
assignments or queries, binds exact producer/effect identities, and validates
potential dependencies before selection. Runtime aggregation unions admission
across declared effects by retained position, preserves repeated positions of
one origin, and schedules ordinary typed programs in the same effect graph.
Generated final inputs use declared projection; Direct final inputs use Skill
stats. The component reuses the existing preparation-readiness fixture builders
and does not publish a second game-data package or evaluator.

### Verified component and next integration

The executable `OwnedSupportEffectPlan` component passes fifteen public Rust
tests. It reuses preparation-readiness fixture builders and proves nonzero
external properties with zero supports and zero queries, two Actions per source,
independent physical copies, Direct final stats, repeated positions and membership
across two effects. Negative cases cover incomplete inventories, owner aliases,
missing final producers, activation, phase ordering and hidden cycles. Four
private dependency tests additionally prove source-reachable cycles, lazy branches,
stage precedence and budgets. Storage tests reject undeclared scopes, wrong
versions, unknown fields, illegal outputs and incomplete program owners.

A/B/A and Rayon worker reuse, shared-budget failure/recovery, all five unchanged
originals and WebAssembly checks pass. Full selected validation is recorded in
[the preserved checkpoint](implementation-history.md#native-source-property-aggregation--b471b4d-2026-10-04).
These tests validate the source relation mechanism; they do not supply complete
real game-data inventories or final-input rules.

The data checkpoint now supplies Partial Ice pre-support/final-level fragments,
six physical support preparations and Exodus's count-one property. Its explicit
V5/V18 release preserves every real owner gap and contains no evaluation bundle.
The finite native path consumes those exact programs with raw inputs, replacing
its placeholder final-level provider. Its item-free, already-admitted boundary
is not proof of general support applicability or complete incoming properties.

Complete actual external contributors, source matching, Amulet copy routing,
supported/final quality and the remaining support mechanics next. Publishing
receiving V3/stages V3 still requires Complete real owners and incoming inventories;
storing Partial V18 rule fragments does not grant that authority. Generated
saved-usage ownership and canonical PoB lifecycle remain independent decisions.
The [implementation plan](implementation.md) records validation and exact release
identity for the data checkpoint.
