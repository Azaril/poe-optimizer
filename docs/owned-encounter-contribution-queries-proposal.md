# Checked Encounter contributions and numeric Enemy queries

Status: **accepted 2026-10-15; generic Actor/Encounter query contract implemented**.
Updated: 2026-10-16. Decider: project owner. Scoped runtime/Import/CLI tests,
the full publication chain, catalog reproduction and strict Clippy pass.
Real numerical Encounter producer bindings remain to be published.

**Reconciliation after the owner's generic-actor direction:** the revised
[Actor identity proposal](owned-existing-enemy-rule-ownership-proposal.md)
makes the scenario Enemy an exact Actor recipient. That absorbs the
numeric Enemy-query permission work below into one Actor query contract.
The implemented explicit Encounter producer origin preserves the actual
selected Encounter owner. The owner approved these as one coherent identity,
local-rule and role-authority contract. Numeric and Boolean queries now use the
unified Actor path; there is no parallel Enemy-specific numeric path.

## Concrete blockers that motivated the extension

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

Both configured source roles belong to the selected Encounter. This ownership
does not mean the new Critical Weakness producer or either final numerical
consumer is already published. Native Engine already instantiated the selected
Encounter's Enemy/Environment programs once, using
`RuleOrigin::Encounter`, the selected Encounter definition and the scenario's
actual Enemy. Before this migration:

- Core's `ContributionOrigin` could not name an Encounter producer.
- Data's checked-query membership validator could not authenticate that origin.
- Engine's checked-query binder accepted reviewed provider, existing-Actor and
  application origins, but could not bind the existing Encounter invocation.
- Checked Enemy reads admitted Boolean Flag groups only. Numeric groups and
  numeric selection on Enemy were rejected even though direct Enemy Stat reads
  and the older numeric reduction mechanism existed.

Relevant code: `owned_rules.rs` in Core, `owned_rules/ordered.rs` in Data, and
`owned_plan/compile/ordered.rs` plus `encounter_and_usage()` in Engine. This
was an owned-model gap; it required no Lua compatibility feature.

## Accepted decision

The **existing checked contribution graph** now has an explicit Encounter
origin and typed numeric queries on the actual scenario Enemy Actor. Keep sources as contributions,
with source identity, recipient identity and membership validated in one place.

1. An Encounter member names its actual definition/program/effect. Bind it only
   to the validated request's selected Encounter and existing Encounter
   invocation; do not create a provider root, duplicate Enemy, or new actor.
   Actor-local scenario-Enemy programs and Environment programs retain explicit
   destination permissions. An unselected known Encounter creates no occurrence; a missing
   or malformed selected producer remains an error or explicit coverage gap.
2. Admit numeric checked groups and checked numeric selection on the exact
   scenario Enemy where the program already has the corresponding Enemy read
   authority. Scenario-Enemy Actor programs and Action consumers are the immediate
   cases. Preserve separate shared-Actor, receiver, Skill and application
   permissions; this decision does not grant arbitrary cross-Actor reads or
   writes, nor introduce Enemy stat receivers.
3. Authenticate the actual contribution recipient. Encounter membership accepts
   the selected Encounter's contribution to `Actor(ScenarioEnemy)`; it grants no
   Player destination authority. Environment programs retain their local
   permissions but are not numerical Encounter-query producers. Player and
   shared-Actor sources retain their own origins.
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
| Extend the existing graph (accepted and implemented) | One source census, recipient model and reduction path; supports configured buffs/debuffs, armour and later encounter mechanics | Required coordinated Core/Data/Engine changes and negative authority tests |
| Derive a separate configured term, then add it to queried contributions | Could avoid the Encounter origin change for one formula | Splits ordinary source membership across computed fields and contribution groups; needs extra rules to prevent omission or double counting; did not solve the former numeric Enemy query gap |
| Use the older direct numeric reduction | Reuses an existing numeric path | Bypasses the newer explicit source-membership contract and continues a path targeted for consolidation |

The accepted implementation follows the previously accepted Actor/reward, exact Skill,
Action and application-group extensions. It is a new public source/recipient
contract, approved with the generic Actor migration under the owner's standing instruction.
It does not imply acceptance of PoB's anomalous critical-MORE function call.

## Implementation and validation after acceptance

- [x] Extend Core/Data with the current-format origin and numeric Actor group
  permissions. Validate owner kind, program/effect existence, contribution kind,
  unit, exact target and ordering. Keep independent owner-specific restrictions.
- [x] Bind the real selected Encounter in Engine without inventing a provider.
  Validate all candidate recipients, including groups not queried by the chosen
  output. Reject duplicate membership, unlisted sources and spoofed origins.
- [x] Exercise checked Encounter and shared-Actor contributions in native
  integration tests, including exact absolute Enemy reads from an Action,
  inactive/missing inputs, Partial membership, wrong roles, duplicate
  applicability and fresh/reused/Rayon execution.
- [ ] Bind a real configured Enemy producer and a distinct additional game
  supplier in one checked graph. Admit their actual condition/source domains;
  generic contract controls do not supply that numerical coverage.
- [x] Prove selection/level/input changes invalidate the bound result; check
  exact fresh/reused/Rayon equality and unchanged source identities.
- [ ] Publish Critical Weakness and its outgoing-critical consumer using real
  imported inputs. Then connect existing Armour `31e0` to physical mitigation,
  applying armour separately to normal and critical hits.
- [x] Rebuild all five imports and preserve their queries, unrelated inputs and
  unresolved coverage: 110 queries and unresolved counts 106/117/109/123/4.
  Exact package/replay inverses, scoped Import/CLI regressions, catalog
  reproduction, the full publication chain and strict Clippy also pass.

The [Actor migration status](owned-existing-enemy-rule-ownership-proposal.md#implementation-and-validation-status)
records the passing local gates; the implementation plan tracks hosted CI
separately. Offering Speed and the existing
Player-defence components remain published; the new contract enables their next
Enemy numerical dependencies without completing them. The first complete build
and all-five parity remain delivery goals. Complete native builds remain **0/5**.
