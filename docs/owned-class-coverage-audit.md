# Remaining Class coverage audit

Status: audited on 2026-10-06 at `81227b8`; updated after the loaded-class witness
and accepted Player slot-read implementation passed. Class closure is withheld.
This audit does not publish game data or change Class coverage.

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

The acquisition-proof gap is now resolved: the original constructor's loaded
`tree.lua` class rows match the authenticated `tree.json` catalogue, and
`characterData` is absent. Original initialization selects the actual class
row. Shared slot-read authority is also implemented and tested. Keep Class
Partial until shared equipment-state producer ownership and its independent
evidence are established; these two completed prerequisites alone are not closure.

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

The new bounded witness `owned_loaded_class_tables` captures the original tree
constructor before and after its metadata additions, plus the original class
initializer. All eight raw class rows and ascendancy metadata match the pinned
catalogue. `characterData` is explicitly absent, and the original base records
retain exact object provenance to the selected `tree.classes[classId]` row.
Constructor-added aliases and start-node IDs are accounted for separately.

Evidence: `runs/owned-loaded-class-tables-source-01/source-jit-{off,on}.json`.
Both reports contain 1,635,614 bytes and SHA-256
`8cbd3841d48b7ac98d9f10668e1ca14f7e80b0a17d9f4bf466786550376103d2`.
All five unchanged originals, an independent repeat of Original05, and fresh
unhooked comparisons pass: 12 complete loads per JIT mode, 24 total, in 86.96s.
Scalar outputs, saved selection and items remain exact. The observation uses
the existing reference bootstrap and no business-method wrappers or copied
initializer. It does not select all eight classes in calculation, inspect the
full output graph, settle warm caches or certify native whole-build parity.

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
- `compile/sources.rs` checks incomplete selector membership. Its equipment
  selection now shares `compile/equipment_slots.rs` with Player reads: authored
  active-loadout occupancy is checked before provider resolution. An unresolved
  or ambiguous equipped item cannot disappear into an empty-slot fallback.
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

The audit originally identified **slot-sensitive input authority**, rather than
arithmetic, as the missing mechanism:

- Before the accepted extension, `RuleReadSource` had no equipment-destination
  or slot-occupancy predicate. Equipment rules can read their own item/modifier/
  quality inputs and computed
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
audit does not recommend another example-build-specific restriction. The follow-up
[slot-state decision](owned-equipment-slot-state-proposal.md), accepted on
2026-10-06, permits a shared Player Actor to read the explicit slot relation and
the selected occurrence's computed Stat/Capability values. It requires reuse of
active-loadout/ambiguity checks and keeps item profile presence separate from
occupancy. Core/Data/Engine now implement that shared resolver,
exact existing-Player invocation authority, computed dependency binding and
stage checks. Focused empty/occupied, ambiguous/unknown, repeated/loadout,
query-independent and Rayon controls pass. No equipment-state formula has yet
been published, so this does not close any equipment-state owner or Class.

## Next bounded producer packet: off-hand structural facts

The next executable slice is the three off-hand branches at
`CalcPerform.lua:273`–`278`, owned by existing shared Player Actor `332a`:

- Empty off-hand is the negation of `Occupied(Weapon2)` for slot `0065`.
- Shield and Focus classifications come from the selected EquipmentUse. Guard
  those reads with occupancy, using ordinary lazy `Select` and explicit false
  only for a proven empty slot.
- Supply the two item classifications as injected capabilities from the pinned
  1,756-base catalogue (193 Shield and 51 Focus rows). `Item.lua:1077` derives
  item type from the base; no item name or source enumeration belongs in Rust.

These are structural equipment facts, not complete modifier-condition truth.
`ModStore.lua:413` also checks inherited conditions and contributed
`Condition:<name>` flags. Keep the latter inventory open and do not publish a
structural result under an identifier claiming its broader closure. Keep both
shared Actor and Class rule inventories Partial.

Authenticate all five unchanged originals, independent fresh/JIT repeats and
empty/Shield/Focus/other-item controls. Original05 has an empty off-hand,
Original01 has a Focus, and Original02's selected swap has a Sceptre. Native
validation must also cover loadout changes, absent classifications, ambiguous
occupants, query independence and Rayon replay.

Defer final Unarmed/Unencumbered and profile substitutions. Raw profile capability
`1d3b` is not final Unarmed authority: `Item.lua:2502`, `CalcSetup.lua:1853`–`1880`
and `CalcPerform.lua:3229`–`3234` have additional branches. Audit the latter's
`env.player.Gloves` read against actual runtime state before importing it as a
native law. No source quirk is adopted by this plan.

## Next actions

1. Done: loaded-class/`characterData` witness, with existing source and native
   attribute/attack proofs retained.
2. Done: accepted bounded slot-state input authority and focused native tests.
   Next use existing provider delivery and shared Actor rules to give unfinished
   ordinary, disabled-weapon, off-hand and special-source conditions explicit
   owners; do not add a marker merely to remove the Class marker.
3. Publish authenticated shared-state producers with contrasting ordinary and
   special-source controls. Preserve Partial refusal for unsupported replacements
   and retain the slot tests independently of game-specific formula evidence.
4. Only then publish a Class-intrinsic closure with no numerical-body changes,
   exact predecessor inverse, independent action/provider/Actor refusal controls,
   and all-five input/query preservation. Complete final attributes and final
   Life remain separate work.
