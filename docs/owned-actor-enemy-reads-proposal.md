# Final critical-result ownership and scenario Enemy reads

Status: stat-owned Player receivers accepted and implemented, 2026-10-14.
The October 7 shared-Actor authority extension below is superseded and will
not be implemented for these results.

## Accepted decision: use existing stat-owned Player receivers

The current architecture already has an owner for common final results:
[stat-owned receivers](owned-stat-receivers.md). Use one explicitly Player-bound
receiver per final incoming-critical Stat, with explicit Stat dependencies
between results. Keep intrinsic/source contributions on the shared Player Actor.
This preserves the actual defending Player; it does not move the calculation to
the Encounter or copy Enemy values onto Player fields.

This is an ownership correction to the unpublished data draft, not an extension
of shared-Actor read permissions. Existing receiver programs can read resolved
scenario Enemy Stats and declared Enemy Boolean Flag queries through the normal
typed dependency graph. Each receiver writes exactly its own final Stat on its
bound Player. Arbitrary cross-Actor writes, numeric Enemy queries and direct
Boolean reductions gain no authority. The alternative is the previously
proposed extension below, which adds a broader shared-Actor read contract.

The static review checked Data's receiver validation and Enemy Flag-query
restriction, Engine's exact Player applicability and Enemy binding, producer
registration, stage/cycle checks and rule/plan identities. This is consistent
with the existing distinction between shared Actor intrinsic contributions and
stat-owned final formulas. The published
[receiver packet](../data/owned/poe2/3887ae68/player-critical-receivers/README.md)
now executes this combined path. Nine native and two CLI tests pass, including
all 52 source observations, exact Enemy bindings, unavailable inputs, lazy
branches, checked overrides and fresh/reused/Rayon equality. Checked all-five
publication preserves every prior coverage declaration and all 110 queries.
Six numeric dependency producers and four Partial supplier query memberships
remain unfinished; this component is not complete critical-effect support or
a complete unchanged build. The [current checkpoint](implementation.md#current-checkpoint-player-incoming-critical-receivers)
records exact identities and validation.

The owner approved this revised ownership direction on October 14. The completed
bounded implementation follows the existing receiver contract:

1. Move only final-result programs to their Stat owners, with explicit Player
   receiver rows. Preserve shared Actor `332a`'s existing Partial inventory and
   every untranslated source/numeric-contributor gap.
2. Rebase the unpublished draft against the current release and allocate new
   IDs from its actual registry. Historical provisional IDs are no longer free.
   Reuse the published configured critical inputs instead of a parallel import
   policy or duplicate presence/value model.
3. Test real Player receivers reading exact Enemy Stats and declared Flag
   queries. Cover wrong type/scope, missing producers, Partial membership,
   competing producers, stages/cycles, lazy branches and unchanged write limits.
   Include ordinary/prepared execution and fresh/reused/Rayon equality.
4. Validate against retained original-source controls and preserve all five
   imports through checked publication. Keep whole-plan, source and final-metric
   completion distinct; receiver ownership alone closes none of those gates.

The following records the original proposal and its trade-off. It is not an
instruction to implement the broader authority as well.

## Concrete blocker

The incoming-critical data draft calculates the defending Player's critical
chance and damage factor on its existing shared Actor owner `332a`. The formula
depends on Enemy flags and resolved Enemy quantities as well as the defending
Actor's evasion, reduction and unlucky flag. Keeping the outputs on that Actor
preserves their meaning when different defenders face the same enemy.

Native execution currently refuses the draft before evaluation:
`existing actor read requires unsupported provider authority`. In
`crates/poe-optimizer-data/src/owned_rules/existing_actors.rs`, shared Actor rules
may read Enemy external inputs and Enemy level, but computed Stat and
ContributionQuery reads are restricted to their own Actor. The accepted Boolean
contribution change admits Enemy Flag queries in ordinary contexts; it did not
extend this separate shared-Actor restriction.

## Original alternative: extend shared-Actor read authority

Allow an explicitly bound existing Player Actor program to read **resolved
Stats on the scenario's Enemy**, and **declared Boolean Flag queries on that
Enemy**. Reuse `RuleEntity::Enemy`, typed Stat definitions and the current
contribution-query registry. No new entity, source projection, DTO, mutable
lookup or second evaluation path is needed.

The scenario already selects exactly one Enemy. Data validates entity scope,
Stat type and query authority; native binding resolves that same concrete Enemy.
All reads join the existing dependency graph, preparation requirements, cycle
checks, coverage checks and bounded-work accounting. Missing producers or
Partial Flag membership remain unavailable. This authority cannot supply a
false flag or zero numeric input.

Keep all writes restricted to the bound Actor. Keep arbitrary provider, owner,
equipment, skill and cross-Actor reads outside this change. It does not admit
numeric Enemy contribution queries, new contribution origins, application
delivery or direct Boolean reductions. Numeric contributor coverage remains
separate work; a computed Stat is only usable when its real producer is resolved.

Use the current development format and rebuild the affected data. There is no
need for a compatibility implementation or another import mode. Review cache
identity implications with the implementation: previously forbidden new data
must not borrow an existing compiled-plan identity.

## Original rejected alternative: Encounter ownership

An Encounter program could perform the calculation and project its result to
the Player. That avoids this read extension, but makes defender-specific
evasion, reduction and unlucky state dependencies of the Encounter owner and
requires additional cross-target output authority. It would also complicate
later use for minions or multiple defenders. Copying Enemy values onto Player
Stats solely to pass the existing validator would hide the relationship and
duplicate data.

## Validation requirements if the original extension is chosen

1. Extend only the explicit existing-Player read allowlist; validate all declared
   readers, including unused owners. Preserve output and numeric query limits.
2. Exercise actual shared Actor binding with resolved Enemy Stats and Flag
   queries, not a fixture that removes the shared Actor applicability record.
3. Prove unavailable results for missing/wrong-scope providers and Partial Flag
   membership, preserve stages/cycles and reject unauthorized direct/numeric
   reductions. Check ordinary and prepared entry points.
4. Execute the unchanged authored critical formula against measured Original05
   source controls, including lazy branches, DOT exclusion, reused scratch and
   Rayon agreement. Publish and reimport all five originals with exact inverses.

The current draft and source witness are independent preparatory work. This
proposal does not make a complete request, close numerical membership, complete
EHP, or authorize changing Original05's saved selection.
