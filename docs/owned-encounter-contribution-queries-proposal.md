# Checked Encounter contributions and numeric Enemy queries

Status: **accepted within the generic Actor migration, 2026-10-15**.
Decider: project owner. Implementation and validation remain pending.

**Reconciliation after the owner's generic-actor direction:** the revised
[Actor identity proposal](owned-existing-enemy-rule-ownership-proposal.md)
makes the scenario Enemy an exact Actor recipient. That absorbs the
numeric Enemy-query permission work below into one Actor query contract.
The explicit Encounter producer origin is still required and is included in
the revised migration; configured contributions must retain their actual
selected Encounter owner. The owner approved these as one coherent identity,
local-rule and role-authority contract. Implement the unified Actor query path;
do not implement or retain the earlier Enemy-specific numeric path.

## Concrete blockers

The next Sniper critical-chance calculation needs the configured Critical
Weakness contribution on the scenario Enemy. The existing imported count
`3390` is an input, not the final contribution: pinned ConfigOptions clamps the
count to 0–20, divides by two, and emits a conditional Enemy `SelfCritChance`
BASE modifier. The source retains this supplier even when its enabling
condition is false. Its observed zero effective contribution cannot prove an
empty channel.

The following physical-mitigation calculation needs the existing configured
Enemy Armour BASE producer `31e0`. That producer already derives its default
from the injected monster table and canonical Enemy level, with a separate raw
override. Reusing it avoids another imported level, armour default, or table.
Other armour suppliers and transformations still need their own coverage.

Subsequent source review also found shared Enemy baseline rules that exist
independently of Encounter selection. Their separate
[applicability decision](owned-existing-enemy-rule-ownership-proposal.md) is
included in the same migration. Encounter membership must not be used to disguise
those baseline owners.

Both are ordinary contributions from the selected Encounter. Native Engine
already instantiates its Enemy/Environment programs once, using
`RuleOrigin::Encounter`, the selected Encounter definition and the scenario's
actual Enemy. However:

- Core's `ContributionOrigin` cannot name an Encounter producer.
- Data's checked-query membership validator cannot authenticate that origin.
- Engine's checked-query binder accepts reviewed provider, existing-Actor and
  application origins, but cannot bind the existing Encounter invocation.
- Checked Enemy reads admit Boolean Flag groups only. Numeric groups and
  numeric selection on Enemy are rejected even though direct Enemy Stat reads
  and the older numeric reduction mechanism exist.

Relevant code: `owned_rules.rs` in Core, `owned_rules/ordered.rs` in Data, and
`owned_plan/compile/ordered.rs` plus `encounter_and_usage()` in Engine. This is
an owned-model gap, not a missing Lua compatibility feature.

## Recommended decision

Extend the **existing checked contribution graph** with an explicit Encounter
origin and typed numeric Enemy queries. Keep actual sources as contributions,
with source identity, recipient identity and membership validated in one place.

1. An Encounter member names its actual definition/program/effect. Bind it only
   to the validated request's selected Encounter and existing Encounter
   invocation; do not create a provider root, duplicate Enemy, or new actor.
   Enemy and Environment program contexts retain their existing destination
   permissions. An unselected known Encounter creates no occurrence; a missing
   or malformed selected producer remains an error or explicit coverage gap.
2. Admit numeric checked groups and checked numeric selection on the exact
   scenario Enemy where the program already has the corresponding Enemy read
   authority. Current-Enemy programs and Action consumers are the immediate
   cases. Preserve separate shared-Actor, receiver, Skill and application
   permissions; this decision does not grant arbitrary cross-Actor reads or
   writes, nor introduce Enemy stat receivers.
3. Authenticate the actual contribution recipient. Preserve Player-bound
   Encounter contributions under the existing Player query contract, without
   treating them as Enemy contributions. No Environment numeric query authority
   is needed for this change.
4. Keep the complete candidate census, including inactive, zero, unknown and
   late effects. Source membership does not complete the Encounter's program
   inventory or the build. Ordered numeric folds still need explicit semantic
   ranks and multiplicity proof; Boolean Any, checked agreement and exact
   Integer sum retain their existing rules.
5. Compile membership once into the existing immutable graph. Worker scratch,
   bounded work, dependency ordering, identity invalidation and parallel
   execution remain unchanged. Rebuild current artifacts; add no development
   format compatibility branch.

Critical Weakness can then remain an Encounter-owned conditional contribution,
and the exact Action consumes it through the same checked query used for other
eligible suppliers. Enemy Armour can similarly combine its configured BASE
with other explicitly admitted sources before mitigation. Neither consumer may
infer missing modifiers from a known baseline scalar.

## Options and trade-offs

| Option | Benefit | Cost or limitation |
| --- | --- | --- |
| Extend the existing graph (recommended) | One source census, recipient model and reduction path; supports configured buffs/debuffs, armour and later encounter mechanics | Requires Core/Data/Engine changes and negative authority tests now |
| Derive a separate configured term, then add it to queried contributions | Can avoid the Encounter origin change for one formula | Splits ordinary source membership across computed fields and contribution groups; needs extra rules to prevent omission or double counting; numeric Enemy query authority is still missing |
| Use the older direct numeric reduction | Reuses an existing numeric path | Bypasses the newer explicit source-membership contract and continues a path targeted for consolidation |

The recommendation follows the previously accepted Actor/reward, exact Skill,
Action and application-group extensions. It is a new public source/recipient
contract, approved with the generic Actor migration under the owner's standing instruction.
It does not imply acceptance of PoB's anomalous critical-MORE function call.

## Implementation and validation after acceptance

- [ ] Extend Core/Data with the current-format origin and numeric Enemy group
  permissions. Validate owner kind, program/effect existence, contribution kind,
  unit, exact target and ordering. Keep independent owner-specific restrictions.
- [ ] Bind the real selected Encounter in Engine without inventing a provider.
  Validate all candidate recipients, including groups not queried by the chosen
  output. Reject duplicate membership, unlisted sources and spoofed origins.
- [ ] Exercise a real configured Enemy producer and a distinct additional
  supplier in one checked graph. Test false/unknown activation, zero values,
  conflicting overrides, incomplete inventories, wrong targets, unselected and
  missing Encounter owners, late writes and dependency cycles.
- [ ] Prove selection/level/input changes invalidate the bound result; check
  exact fresh/reused/Rayon equality and unchanged source identities.
- [ ] Publish Critical Weakness and its outgoing-critical consumer using real
  imported inputs. Then connect existing Armour `31e0` to physical mitigation,
  applying armour separately to normal and critical hits.
- [ ] Rebuild all five imports and preserve their queries, unrelated inputs and
  unresolved coverage. Update the implementation and cleanup records at the
  runtime and numerical publication checkpoints.

Offering Speed is now published. Independent Player-defence work and source
supplier review can continue during the coordinated migration. The first complete
build and all-five parity remain the
delivery goals; this proposal closes no numerical or whole-build gate itself.
