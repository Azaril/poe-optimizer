# Shared Actor reads of the scenario Enemy

Status: proposed, not implemented. 2026-10-07.

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

## Recommended decision

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

## Alternative

An Encounter program could perform the calculation and project its result to
the Player. That avoids this read extension, but makes defender-specific
evasion, reduction and unlucky state dependencies of the Encounter owner and
requires additional cross-target output authority. It would also complicate
later use for minions or multiple defenders. Copying Enemy values onto Player
Stats solely to pass the existing validator would hide the relationship and
duplicate data.

## Implementation and validation gate

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
