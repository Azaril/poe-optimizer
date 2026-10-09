# Action damage stages

Damage is computed for an exact action occurrence and damage type. Intrinsic
weapon or skill values, added damage, conversion/gain, modifier scaling, critical
variants, mitigation and final aggregation are separate stages. A result from an
earlier stage is not final hit damage or DPS. The implementation is native Rust
executing injected typed rules; PoB remains an offline source/reference adapter.

## Input and producer ownership

Game definitions own coefficients, units, rounding policies and applicability.
The build supplies allocations, equipment, skills, supports and scenario choices.
Neither the evaluator nor its worker scratch may contain fixture-specific skill
names, build IDs, selected nodes or observed output values.

Modifiers enter typed contribution channels through their actual producers.
An existing semantic channel should be reused before introducing a new one.
Player modifiers intended for owned minions require explicit delivery to the
receiving actor; they do not become general player damage. An action consumes
only modifiers applicable to that action and the declared critical/noncritical
variant. Command-only, spell-only and weapon predicates cannot be discarded or
treated as globally applicable merely because one reference uses Basic Attack.

Each reduction retains complete incoming-membership requirements. Identity zero
or one is valid for a proved empty channel, never for missing source coverage.
Source-field absence, an authored default, a zero contribution and an unresolved
producer are distinct facts. Native tests may close a finite component world;
publishing those programs must retain the real definition's Partial coverage.

## Numerical boundaries

1. Construct coherent weapon/skill source endpoints using their own level/profile
   rules and any source-specific additions, then select the source explicitly.
2. Combine the selected source endpoints and applicable added damage. Apply the
   declared added-damage modifiers and base coefficient in their specified order. Preserve
   intermediate precision unless the injected rule declares a rounding step.
3. Resolve conversion and gain across the full declared damage-type domain.
   Conversion priority, normalization and simultaneous application are game data
   semantics; a single-type shortcut requires evidence for that restricted domain.
4. Apply the receiving damage-type and generic increases, multiplicative factors
   and separate minimum/maximum factors. Produce named intermediate endpoints at
   the declared rounding boundary.
5. Apply subsequent hit factors, critical probability/multipliers, lucky weighting,
   mitigation, damage taken, hit chance, timing and build usage/population through
   their separate dependencies. Mitigation that depends on hit size must see the
   appropriate critical/noncritical range.

Modifier aggregation itself can contain a rounding boundary. The current source
observer's quality-1 control combines parent quality and Gigantic into MORE 1.21,
not the unrounded product 1.212. The source rounds the local store/stat-name
product before combining ancestor stores. Author that verified grouping and
rounding explicitly; do not assume every MORE channel is an unrestricted product
of independent factors. This behavior is separate from effect-family Maximum.

Current typed scalar operations can express the first proposed ordinary physical
range consumer. Do not add a game-specific Core opcode or copy PoB's cfg object
unless a demonstrated semantic requirement cannot be represented by the existing
data contract. The owner has accepted the general
[effect-application model](owned-effect-applications-proposal.md) for delivery,
recipient scaling and stacking. It is implemented through the same typed graph.

## First integration and acceptance

The current investigation targets the original Skeletal Sniper Basic Attack's
noncritical physical range at PoB's calcDamage return, before later hit factors
and mitigation. Existing intrinsic Action channels 3212/3213, minion damage
delivery 1d33/1d34 and parent quality 001d are candidate reusable dependencies;
their source correspondence and completeness must be verified before consumption.

The reference witness must record the original call's source endpoints,
coefficient presence/precedence, BASE/INC/MORE membership and source lineage,
conversion/gain channels and matrices, action conditions, actual returned range
and later hit factor. Include ordinary quality, passive damage, excluded Command
modifiers, Gigantic and valid parsed added/conversion/gain controls. Observe both
JIT modes and reused state while preserving original functions and outputs.

Acceptance requires actual authored producer → actor/action → range execution
against those measurements, including incomplete-producer rejection, independent
occurrences, scratch reuse and parallel workers. An inferred aggregate multiplier
or a fixture-fed final endpoint does not satisfy this integration. Preserve all
five original selections and 110 queries, report their real finalization results,
and keep the final-summon-level/readiness gaps explicit until their own producers
and contracts are implemented.
