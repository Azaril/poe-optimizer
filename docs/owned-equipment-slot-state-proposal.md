# Equipment slot reads for shared Player state

**Status:** Accepted and implemented on 2026-10-06. Game-state producers remain
unpublished; no IDs allocated and no Class coverage changed.
**Date:** 2026-10-06.
**Decision owner:** Project owner.

The [Class coverage audit](owned-class-coverage-audit.md) identifies a missing
input boundary for shared Player conditions. An occupied caster item can supply
no attack profile, an empty hand has no equipment occurrence, and the same
template can appear in different hands. Neither item names nor a queried Action
can determine shared Actor state.

## Accepted decision

Add a bounded, read-only **Player equipment-slot relation** to shared Actor
programs. Resolve an explicit owned slot against the candidate's authored
equipment and active loadout, reusing the existing action-source selector's
occupancy and provider checks. Expose presence and computed EquipmentUse values;
leave item parameters, item-local mechanics and modifier delivery with their
existing providers. This is one relation in the existing evaluation graph, not
a second build model or a synthetic equipment/action occurrence.

An illustrative current-format DTO is:

```text
RuleReadSource::PlayerEquipmentSlot {
    slot: EquipmentSlotDefId,
    read: Occupied | Stat { stat: StatDefId } | Capability { capability: CapabilityDefId }
}
```

The implemented DTO uses `PlayerEquipmentSlotRead` for these three alternatives.
The public semantic decision is the cross-entity read
authority and its absence behavior. Initially permit the
read only in an explicit existing-Player Actor application. Do not implicitly
apply it to owned minions, source-property scopes or arbitrary provider programs.
The Stat/Capability must declare EquipmentUse scope and its exact type/unit.

## Options compared

| Question | Current equipment occurrence reads its destination | Shared Actor reads the slot relation — accepted |
| --- | --- | --- |
| Exact identity | A predicate such as `EquipmentDestinationIs(slot)` binds to the invocation's EquipmentUse; identical templates in opposite hands differ correctly. | The explicit slot resolves to the exact active EquipmentUse; identical templates remain independent. |
| Existing delivery | Reuses EquipmentUse/Modifier → Player contributions directly. | Reuses those programs for computed values and modifier effects; no new copy of their raw inputs. |
| Empty hand | No occurrence executes. Requires exhaustive slot-specific contributor membership and an explicit empty reduction. | Complete authored relation directly proves zero active occupants. No invented zero contribution is needed. |
| Ambiguity | A destination predicate alone returns true for both occupants. Additional occupancy validation or a guarded count domain is required. | The existing selector rule already refuses multiple active occupants; reuse it. |
| Data/graph cost | Adds slot-sensitive delivery, count/presence channels and membership obligations across applicable templates, even for a simple empty-hand question. | A small number of shared reads; candidate binding resolves each distinct requested slot once and reuses immutable dependencies. |
| General utility | Useful later for genuinely slot-dependent local modifiers. It does not alone solve shared state. | Directly addresses shared hand/profile/condition state and avoids Action-demand dependence. |
| API consequence | New read authority in equipment contexts, plus a separate empty/ambiguous relation strategy. | New bounded cross-entity authority in shared Actor contexts. |

Destination predicates remain a valid future feature when an item's own effect
depends on placement. They are not recommended as an indirect representation of
shared slot occupancy. The recommendation prioritizes explicit empty-hand and
ambiguity semantics over minimizing the number of new enum variants.

## Accepted relation semantics

1. **Select by slot and active loadout.** Inspect the candidate's complete authored
   `EquipmentUse` relation before filtering through successful providers. Shared
   uses and uses containing the selected weapon loadout participate; inactive
   alternatives do not. Never choose by item/template ID, iteration order or a
   saved source UI selector.
2. **Preserve uncertainty.** Zero authored active occupants yields `Occupied =
   false`. One yields true only after the normal destination/schema/provider
   checks. An unresolved occupant stays unresolved; more than one stays
   ambiguous/unresolved. Missing slot definitions are invalid, not empty.
3. **Read only the chosen occurrence.** Stat/Capability reads target that exact
   EquipmentUse. Missing producers remain unresolved. An empty slot has no such
   value; there is no implicit numeric zero, false capability or alternate
   profile. Existing lazy `Select` can branch on Occupied before demanding the
   value. Incomplete contributing domains retain normal whole-plan refusal.
4. **Separate occupancy, profile and legality.** An occupied caster slot can have
   a known false attack-profile capability. An unoccupied off-hand may be blocked
   by a two-handed item. `Occupied = false` means no authored use, not that the
   slot can be equipped or that a skill may attack. Two-handed restrictions,
   exceptions, disabled weapons, effective hand state and replacements require
   injected game rules and their own coverage; this relation certifies none.
5. **Preserve provider ownership.** Raw item level, quality, modifier rolls and
   parameter slots stay readable only through their established exact provider
   authority. The shared Actor sees declared computed outputs, not arbitrary
   fields or another occurrence's parameter storage.
6. **Remain independent of queries.** Bind through the explicit Player Actor
   application even if no Player Action is requested. Additional Actions must
   not change the resulting state or multiply its producers. Edits, loadout
   switches and repeated uses rebind through the existing candidate plan.

All selected computed reads join the ordinary producer, readiness and cycle
checks. A computed equipment output that depends back on the Actor condition is
not permission to read a mutable intermediate; its stages must be explicit or
the cycle must reject. Slot indexes and validation work remain bounded by the
existing request/plan limits, with no equipment scan in the numerical hot path.

There is currently no general structural uniqueness or blocked-hand rule in
`EquipmentSlotSchema`: it declares scope. The build's structural checker validates
references/scopes/containment, while the current action selector performs its own
active-slot collision check. Implementation must reuse a single bounded relation
resolver for both callers; it must not assume that earlier structural validation
already proved unique occupancy or game equipment legality.

## Decision scope and implementation

Already accepted: one occurrence graph; exact source/provider ownership; injected
game data; shared Actor rules; distinct equipment presence and attack-profile
facts; explicit unresolved values; query-independent build state; and preserving
mandatory coverage. These justify reuse of existing mechanisms, not silently
adding new read authority.

**Accepted on 2026-10-06:** explicit shared Player Actor programs may read the
validated Player equipment-slot relation and its selected occurrence's computed
Stat/Capability values, with the absence/ambiguity rules above. The project owner
selected the shared Player slot read in direct response to this proposal. This
expands the current same-Actor read boundary. It does not approve a game-specific
unarmed formula, complete any Class, or adopt source cache/UI behavior.

Core/Data/Engine now implement this relation in current operations V21. The
cold resolver in `owned_plan/compile/equipment_slots.rs` is shared with Action
source selection and caches each requested slot. Data validates every reader,
including unused owners and packages without an Actor application inventory.
The invocation binder separately requires the actual existing Player: listing
an Actor definition does not authorize an owned minion using that definition.
Computed reads join ordinary concrete dependencies and EquipmentUse stage
channels; occupancy does not fabricate a computed producer.

Validation passes: one Core wire test, five new Actor storage tests, two new
stage tests and eight Engine tests. The Engine checks empty/occupied caster
slots, identical templates in opposite hands, active/inactive alternatives,
ambiguous/unknown occupants, missing outputs, raw compiler authority, cycles,
stage order, query independence, A/B/A scratch reuse and Rayon workers. Existing
Action selection, Actor, readiness and participation regressions also pass.
Evidence is under `runs/owned-player-slot-*.log`.

No current game package uses the new reads yet. Next author the ordinary
shared-state inputs and original-call evidence before reconsidering Class
closure. Two-handed legality and unsupported replacements remain incomplete.
No compatibility version or duplicate implementation was added. The whole-rule
reader census consumes the existing bounded work budget; diagnostic resource
counts can change even where stored rule bytes and content identity do not.
