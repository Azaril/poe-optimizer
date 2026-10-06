# Remaining Class coverage audit

Status: audited on 2026-10-06 at `81227b8`; closure withheld. The follow-up shared
Player slot-read contract was accepted on 2026-10-06; implementation is pending.
This audit does not publish data or change Class coverage.

The checked baseline is `runs/owned-player-rule-ownership-01/package`, input
`344d3a2c278844c6ea9b2d1f2e27edde492e23392f8500aec5f3f3baf533051f`.
All eight Class owners `0a1e`–`0a25` still have
`tree-game-rules-not-converted`. No IDs are reserved and no Class closure is
changed by this audit. Complete original-build evaluation remains 0/5.

## Conclusion

The remaining Class programs contain the expected intrinsic facts: class base
attributes and class intrinsic attack values. Separate action, skill, item and
passive coverage gates are real and mandatory. However, those action gates do
not account for all shared equipment-derived Actor state. The source derives
`Unarmed`, `Unencumbered` and `HollowPalm` conditions outside the shared
initialization inventory documented for Actor `332a`. Merely retaining that
Actor's generic Partial marker would not establish ownership of these branches.

There is also an acquisition-proof gap: the class converter authenticates
`tree.json`, whereas the source constructor normally loads `tree.lua`, and
calculation prefers a loaded `characterData` table when present. Existing
retained reports do not explicitly bind that alternative to the complete loaded
eight-class catalogue. Keep Class Partial until this small proof and the shared
state ownership are established.

## Class facts and metadata

The following source paths are relative to the pinned
`vendor/path-of-building-poe2` checkout.

| Source or owned evidence | Finding and responsibility |
| --- | --- |
| `src/Modules/CalcSetup.lua:827`–`832` | The selected class contributes `base_str`, `base_dex`, `base_int`. These are the only values read from the local `classStats` table. |
| `src/Modules/Data.lua:626`–`636` | Complete unarmed catalogue: nine rows and five fields per row. The owned namespace admits eight classes; source class 0 is explicitly excluded by the intrinsic-attack policy. |
| `data/owned/poe2/3887ae68/class-bases/policy.json` | All eight source class records have exactly seven reviewed fields: three bases, `integerId`, `name`, `background`, and `ascendancies`. Count-cutover data preserves the numerical class facts while sending each base to both attribute passes. |
| `data/owned/poe2/3887ae68/intrinsic-attack/` | Authenticated catalogue and field policy supply Actor baseline Stats `1d35`–`1d38`; the constant source type marker is not a native equipment record. These values are not final attack inputs. |
| Current `rules.json` | Each Class has exactly `class-base-contributions` and `intrinsic-attack-baseline`. Intrinsic Life is absent from all eight and resides once on shared Actor `332a`. |
| Current Class schemas | All seven declaration inventories are Complete and empty. Level range, ascendancy membership and implicit-root relation remain explicit. Different implicit roots retain their own independent coverage. |

Metadata does not create additional intrinsic numerical Class programs:

- `PassiveTree.lua:101`–`140` indexes classes and ascendancies, aliases the
  ascendancy catalogue, and constructs import/name maps. `integerId`, `id`,
  `internalId`, names and backgrounds serve identity, import or presentation.
  `PassiveTreeView.lua:637`–`651` uses ascendancy `replace`/`replaceBy` for tree
  presentation. Those source labels must not become runtime native dispatch.
- `PassiveTree.lua:227`–`245` and `PassiveSpec.lua:655`–`743` establish Class and
  ascendancy roots. These belong to the existing schema/topology relations,
  with root rules and declarations checked independently. The completed
  Witch/Sorceress root proof does not close other roots or Classes.
- `PassiveSpec.lua:1484`–`1494` selects class/ascendancy-dependent node variants.
  This is passive-owner behavior. The native converter
  `crates/poe-optimizer-import/src/owned_passive_views.rs:278`–`400` uses typed
  Class/Ascendancy reads and preserves class precedence. Concrete current
  examples: source nodes 1755/4739 map to Complete owners `0c76`/`1585` with
  `passive-view`; 1143/3823/4313 map to Partial owners `0aaf`/`12b0`/`1417`.
- `CalcSetup.lua:1533`–`1534`, `1620`–`1632` also consume class identity for the
  Forbidden Flame/Flesh granted-ascendancy mechanism. This is item/modifier and
  grant authority, not an additional base Class value. This audit has not proved
  that mechanism's canonical admission or numerical implementation.

The reader census followed `classStats`, `env.classId`, `curClassId`, class and
ascendancy metadata aliases, class start maps, and the complete
`unarmedWeaponData` references in source Modules/Classes. It is a bounded source
audit, not a claim that arbitrary source code cannot dynamically add fields.

## The loaded `characterData` alternative

`PassiveTree.lua:69`–`98` first reads `TreeData/<version>/tree.lua`, evaluates it,
and copies its returned top-level entries. Only when that file is absent does
it convert its alternate JSON source. `CalcSetup.lua:827` selects
`tree.characterData[classId]` before `tree.classes[classId]`.

The complete pinned `0_5/tree.json` top-level inventory has no `characterData`
or `alternate_ascendancies`. This proves the JSON domain, not the loaded Lua
object's identity or absence. The existing class-start-root report intentionally
captures the Witch/Sorceress root. The intrinsic-Life report captures all class
rows and the shared initializer, but does not explicitly record the preferred
class-table branch. Neither report should be extended by interpretation.

The smallest additional evidence is a bounded capture after the original tree
constructor: all eight actual class rows, the explicit presence/absence and
identity of `characterData`, and the exact table used for the original base
initializer. Authenticate both source files and compare the selected fields to
the existing finite class catalogue. A full new calculation harness or copied
formula is unnecessary.

## Actual action and provider gates

The current release contains five action-output routing records. Every routes
inventory and every source-selector inventory is Partial. There are no native
rule reads or routes referencing Class intrinsic attack Stats `1d35`–`1d38` in
this release; only the Class programs write them. This is an enumeration of the
current complete artifacts, not a claim that those channels are unused forever.

The independent gates are enforced by the normal plan:

- `crates/poe-optimizer-engine/src/owned_plan/compile.rs:2237`–`2265` emits
  `MissingRouting` for an absent action-output routing record and
  `PartialRouting` for an incomplete routes inventory.
- `compile/sources.rs:45`–`63` checks incomplete selector membership. Its
  equipment-selection path at lines 86–141 checks authored active-loadout
  occupancy before provider resolution. An unresolved or ambiguous equipped
  item cannot disappear into an empty-slot fallback.
- `compile.rs:1350`–`1377` checks each actual discovered provider's rule owner.
  Missing/Partial skill, item, modifier, passive or ascendancy ownership remains
  mandatory independently of the Class owner.
- `compile.rs:512`–`529` includes provider, Actor, receiver, action, routing and
  ordered-query obligations before computing plan completeness.
  `support_effects.rs:386`–`398` refuses the checked evaluation when those
  obligations are incomplete.

Examples of rejected replacement domains are concrete: Punch
`MeleeUnarmedPlayer`/Skill `0276`, Crackling Palm `00e6`, Gelid Palm `01b2`, and
the mapped Concoction skills have Unmapped skill schemas. Their known source
identity does not grant an executable action. Ordinary weapon-profile presence
Capability `1d3b` is also only a template fact; it does not prove concrete item
activation, skill compatibility or selected-source policy.

These gates preserve unfinished **action** mechanics. They are not a substitute
for separately accounting for shared Actor conditions used outside an attack.

## Remaining shared equipment-state responsibility

| Original source branch | Consequence that must retain separate ownership |
| --- | --- |
| `CalcSetup.lua:1853` | A caster item may occupy Weapon 1 while lacking attack weapon data, selecting the intrinsic profile. Empty hand and intrinsic profile are different facts. |
| `CalcSetup.lua:1855`–`1879` | Dual-wield construction, Hollow Palm off-hand construction, `HollowPalm`, and Facebreaker substitutions depend on exact hand state and contributed effects. |
| `CalcPerform.lua:264`–`285` | `UsingShield`, `UsingFocus`, `OffHandIsEmpty`, `Unarmed` and `Unencumbered` are Actor condition inputs. They can affect modifiers beyond a selected attack. |
| `CalcPerform.lua:3229`–`3234` | `DisableWeapons` changes the effective profile and Actor conditions later in the pipeline. |
| `CalcOffence.lua:2526`–`2559` | A skill-specific unarmed flag replaces either attack pass's numeric profile. This remains action-source policy. |
| `CalcActiveSkill.lua:513`–`542`, `887`–`892` | Hand compatibility and Hollow Palm damage use the prepared profile/state; baseline values alone do not select these branches. |

The accepted [shared Actor ownership](owned-existing-actor-rule-ownership-proposal.md)
inventory currently identifies shared initialization in `CalcSetup.lua:26`–`114`
and `834`–`899`. Actor `332a` still owns exactly the intrinsic-Life program with
`shared-player-initialization-not-converted`. This audit does not silently expand
that marker to certify the equipment-state branches above.

## Reusing existing delivery: what works and what is missing

No new evaluator or source-language facility is needed for the numerical and
Boolean mechanics. EquipmentUse or exact Modifier programs already contribute
to the Player. The cold-item-delivery program is a concrete checked example.
Discovery resolves actual selected-loadout equipment and preserves unresolved
providers; repeated uses retain distinct identities. A shared Actor program can
consume those typed contributions and derive a Boolean Stat using existing
arithmetic/comparison operations. The current Actor validator explicitly admits
same-Actor Stat, Capability and contribution reads
(`crates/poe-optimizer-data/src/owned_rules/existing_actors.rs:88`–`112`).

That supports data-authored state producers once the exact incoming domain is
proved. Examples include a modifier's explicit disable contribution, or a
template property delivered from an actual equipment use. An empty reduction
must still have complete membership and normal whole-plan coverage; no producer
absence may become an invented false condition.

The remaining obstacle is **slot-sensitive input authority**, not arithmetic:

- `RuleReadSource` has no equipment-destination or slot-occupancy predicate.
  Equipment rules can read their own item/modifier/quality inputs and computed
  values, but cannot distinguish one identical template used in Weapon 1 from
  the same template used in Weapon 2. Equipment providers bind template/modifier
  owners; destination slots do not instantiate a separate slot-rule owner.
- Equipment-template receivers reuse those same exact occurrences; they do not
  add a slot predicate. Template identity is not a safe proxy for a hand.
- Ordered-contribution equipment slot lists give semantic **ordering ranks**.
  They do not filter one hand into a separate group: an unlisted relevant use
  rejects the query. They must not be repurposed as a hidden occupancy selector.
- The existing action-source selector does prove exact slot occupancy, but its
  decision belongs to a demanded Action. Projecting it into shared Player state
  would make that state depend on queried actions and could duplicate producers.
  Creating an artificial action for this purpose would violate the one-graph
  ownership design.

For a proven template restricted to one slot, existing delivery may suffice for
a bounded component. It does not solve the general hand-state problem, and this
audit does not recommend another example-build-specific restriction. Therefore
a full ordinary hand-state implementation cannot currently be expressed through
data delivery alone. The follow-up
[slot-state decision](owned-equipment-slot-state-proposal.md), accepted on
2026-10-06, permits a shared Player Actor to read the explicit slot relation and
the selected occurrence's computed Stat/Capability values. Implementation must
reuse the existing active-loadout/ambiguity checks and keep item profile presence
separate from occupancy. The accepted contract remains unimplemented; it does not
close any equipment-state owner or Class.

## Next actions

1. Obtain the small loaded-class/`characterData` witness described above; retain
   existing source and native attribute/attack proofs.
2. Implement the accepted bounded slot-state input authority, using existing
   provider delivery and shared Actor rules wherever possible. Give unfinished
   ordinary, disabled-weapon, off-hand and special-source conditions explicit
   owners; do not add a marker merely to remove the Class marker.
3. Test occupied caster versus empty hand, both hand destinations for one
   compatible template, active/inactive loadouts, missing/ambiguous providers,
   and unrelated action selection. Preserve Partial refusal for unsupported
   special replacements.
4. Only then publish a Class-intrinsic closure with no numerical-body changes,
   exact predecessor inverse, independent action/provider/Actor refusal controls,
   and all-five input/query preservation. Complete final attributes and final
   Life remain separate work.
