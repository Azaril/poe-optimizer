# Checked queries over stacked application contributions

**Status:** Accepted and implemented in Core/Data/Engine operations V25 on
2026-10-08. Real mixed damage-consumer data adoption remains pending.
**Date:** 2026-10-08.
**Decider:** Project owner.
**Scope:** Let the existing contribution graph consume the result of an effect
application's stacking group, preserving its exact recipient and source evidence.

## Concrete blocker

The joined Sniper graph now executes the published Pain Offering application
with imported activation, item-prepared level and calculated source/recipient
scaling. It emits 62% increased damage per recipient at final level 22. Two equal
Offering occurrences retain their identities but emit one non-stacking result
per recipient. Independent level changes select the stronger result.

The selected passive component independently delivers 68% increased minion
damage. The next consumer must combine the appropriate contributions and carry
them to the physical damage calculation. The expected 130% subtotal for these
two components is a validation control, not a complete damage-domain claim or a
value to put in the evaluator.

Applications already use the same effect DAG. The missing piece was *checked
membership*: Core's previous `ContributionMember` named only an ordinary
owner/program/effect plus its source origin. Data requires that program to exist
in a definition owner. Engine's application compiler instead emits a synthetic
group keyed by exact recipient, stacking family and modifier. Its diagnostic
owner/program fields do not name an authored definition program. Treating them
as one would fabricate ownership and obscure the distinction between individual
candidates and the contribution after stacking.

Relevant code:

- [Current member contract](../crates/poe-optimizer-core/src/owned_rules.rs).
- [Data membership checks](../crates/poe-optimizer-data/src/owned_rules/ordered.rs).
- [Application group production](../crates/poe-optimizer-engine/src/owned_plan/compile/effect_applications.rs).
- [Exact runtime membership](../crates/poe-optimizer-engine/src/owned_plan/compile/ordered.rs).

## Accepted decision

Give members of the existing checked query a typed producer address:

- **Program effect:** the current owner, program, effect and explicit origin.
- **Application group:** stacking family and modifier, with an explicit reviewed
  set of application/effect declarations allowed to produce that group.

Retain one contribution query, reduction and coverage system. The second variant
names the contribution *after* stacking, never every candidate as an additive
source. The query's stat and contribution kind must match the declared group.
Its concrete recipient is bound from the existing application graph and must
match the channel being read; it is not a user-supplied recipient substitution.

At cold validation, authenticate each named application/effect and stacking
mapping, the common channel, recipient capability and reduction. Census every
potential matching declaration and concrete group, including unread, inactive,
neutral and unavailable sources. A missing or extra unreviewed declaration must
fail membership rather than disappear because it produced no useful value.
Partial application or query coverage remains unavailable. A complete-empty
query cannot hide a potential application contribution.

Each concrete group has one explicit position in a numeric fold. Reuse existing
ordering semantics; inapplicable equipment positions are empty/zero. Neither
candidate order, winning instance ID nor discovery order supplies that position.
Separate recipient occurrences stay separate. Preserve all candidate identities
and tied winners for explanation without counting them multiple times.

False activation continues to skip unavailable strength inputs; absent activation
and unknown candidates remain unresolved. An inactive group contributes absence,
not an authored zero. Existing stage dependencies, cycle checks, bounded cold
planning and worker scratch apply unchanged. This does not add a stacking policy,
support-delivery origin, inherited Skill authority or source-specific opcode.

## Options and trade-offs

| Option | Benefit | Cost |
| --- | --- | --- |
| Typed producer addresses in the existing query (recommended) | One checked reduction and membership model for ordinary effects and application results; supports future buffs/debuffs without fabricated program owners | Changes the public member format and requires rebuilding current authored fixtures/artifacts and validating both producer families |
| A separate typed read of a stacked application group | Smaller initial change to ordinary member data; directly expresses the group read | Adds another read/binding/completeness path and makes mixed reductions responsible for coordinating two APIs; would need consolidation or continued maintenance |

Do not add an ordinary definition program solely to impersonate the synthetic
group. Do not sum candidate values before Maximum, select one tied winner as the
group's owner, or implement the 62 + 68 example in Rust content code.

No development-format compatibility branch is required. Change the current
format, rebuild maintained artifacts and invalidate affected identities/caches.
Retain independent source evidence. Source labels, Lua modifier-store positions
and PoB object identities remain offline provenance.

## Implementation and acceptance

1. Add the typed member address and checked Data validation. Rebuild maintained
   data/fixtures in the current representation; preserve all five imported builds.
2. Bind application-group members in the existing cold planner and late support
   validation. Workers consume the same reduction indices as ordinary queries.
3. Cover more than Offering: synthetic independent families, multiple application
   definitions sharing a family, repeated source and recipient occurrences,
   duplicate/missing membership, wrong channel/recipient, unread/inactive/zero
   writers, Partial coverage, empty domains, tied winners and unknown candidates.
4. Verify explicit ordering, stage/cycle refusal, bounded work, storage
   permutations and fresh/reused A–unknown–B–A/four-worker execution.
5. Publish a reviewed owned consumer for the actual selected passive and Offering
   contributions, validate its subtotal against retained source evidence, then
   continue to the physical damage endpoint. Preserve remaining damage-domain,
   source-admission and full-build coverage obligations.

This is the next numerical consumer boundary for the closest build. No further
approval is required for this contract. Composed-support discovery remains
independent accepted work; the next implementation checkpoint should connect
stacked applications to checked queries before returning to unrelated scalar
components.

## System walkthrough and implementation handoff

A contribution is a typed value attached to a concrete recipient, a Stat and a
contribution kind. A query declares which producers belong to that channel and
how their values reduce. It is not a request to search for all similarly named
modifiers, and it cannot establish complete coverage from the values it happens
to observe.

Applications precede aggregation. Each selected source occurrence is paired with
eligible recipients. Activation gates strength evaluation. Candidate effects then
enter a group identified by recipient, family and modifier. The existing Maximum
operation produces one contribution from that group, retaining tied source
identities for explanation. An inactive group produces absence; an unresolved
candidate can prevent a known result.

The approved query extension consumes that one group result alongside ordinary
program effects. It never adds the candidates again. For the selected Sniper
components, two equal Offerings yield one 62% contribution per recipient, which
can then combine with the applicable 68% passive component. These are retained
validation observations, not rule literals or complete damage-domain coverage.

The implementation layers remain:

| Layer | Responsibility |
| --- | --- |
| Core | Typed producer address and declaration contract; no game-specific calculation |
| Data | Authenticate application/effect mappings, channel, recipient capability, reduction and complete potential-declaration census under bounded validation |
| Engine planning | Bind exact recipient groups and ordinary occurrences; verify membership and numeric positions before values, including late support expansion |
| Engine execution | Use the existing dependency graph, reductions and per-worker scratch; no new aggregation engine |
| Offline reference | Acquire and validate game rules against pinned PoB; no Lua state in native execution |

Before this implementation, `compile/ordered.rs` indexed members by
owner/program/effect and rejected application groups at the ordinary-provider
origin check. `compile/effect_applications.rs` already emitted a
`RuleOrigin::EffectApplicationGroup` with exact recipient, family and modifier.
The new binding uses that typed origin; the synthetic node's diagnostic
owner/program fields are not definition-side ownership.

Both initial planning in `owned_plan/compile.rs` and late binding in
`compile/support_suffix.rs` call `Sources::validate_inventory`. Preserve that
shared gate. Validate declarations even if a source or recipient is not selected;
validate concrete groups even when unread, inactive or neutral. Declaration
completeness and runtime availability are separate checks.

The current member format now has a tagged `producer` plus `order`.
`ProgramEffect` retains the ordinary definition/program/effect/origin;
`ApplicationGroup` carries family, modifier and exact declaration membership.
The bounded Data census and shared Engine binding gate implement both variants.
Execution still uses the existing reduction indices and application operations.

Operations V25 uses plan domain `owned-effect-plan-v22`. Earlier operation
subsets use the same current wire format and reject application-group members;
they are capability subsets, not old-format parsers. The maintained data packets
and canonical package's 2,017 ordinary members have been re-expressed and their
authoring commitments rebound. No prior-member deserializer is retained.

The new generic tests cover mixed ordinary/application channels, independent
families, duplicate declarations sharing a family, source ties, repeated recipient
groups, missing/extra/duplicate members, wrong channels/recipients, unread/inactive/
zero sources, unknowns, Partial coverage, ordering, work bounds, stages, cycles,
storage permutations and fresh/reused/four-worker execution. The next numerical
step is real data adoption, not another contribution API.
