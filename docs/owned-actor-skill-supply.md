# Actor-owned skill supply

**Status:** Proposed; requires owner review before Core implementation.

**Date:** 2026-09-24

**Decision owner:** Project owner, with implementation review against the existing owned contracts.

## Problem and decision

The owned model can identify a summoned actor and select one of its declared outputs. It cannot yet supply an ability occurrence inside that actor. `ActorSlotSchema.skills` is potential membership; the planner deliberately reports `UnresolvedActivation` for its members. Importing a Basic Attack selector therefore does not establish an active Basic Attack skill or numerical coverage.

Recommend **an explicit owned actor definition with provider declarations, reached through the existing actor grant path**. Add `ActorDefId`, `ActorSchema`, and `SlotOwnerDefId::Actor`, and let an actor slot explicitly reference that definition in a new schema version. Keep `ProviderRoot`, `ProviderKey`, `OwnedActorKey`, `GeneratedSkillKey`, and `ActionSelection` occurrence identities unchanged. This is a proposed semantic extension, not an implemented feature.

An actor definition is a reusable calculation template. An actor slot identifies a particular population produced by a particular provider. Two summoners using the same template still produce distinct actors. Neither identity represents each individual simulated skeleton; population count and usage remain explicit mechanics.

## Recommended declaration and occurrence model

Conceptually, the new contracts are:

```text
ActorSchema {
    declarations: DeclaredSlots
}

ActorSlotSchema {
    skills: DeclaredSet<SkillDefId>,        // existing potential membership
    outputs: DeclaredSet<ActionOutput>,    // existing selectable output ports
    provider_definition: ActorDefId        // explicit new-version binding
}
```

The precise wire representation must preserve the old version: omission in an old package means **actor supply unconverted**, never Complete-empty. A new-version actor slot may also explicitly remain unconverted. It must not acquire an actor definition by name matching or because a catalog happens to contain one.

Actor-owned `GrantSlot` and `SkillGrantSlot` declarations belong to `SlotOwnerDefId::Actor(actor_definition)`. They are not added to the summoning Skill's declarations. Entering an actor grant exposes this actor definition's declarations plus the slot's explicit legacy output ports; it does not expose declarations from the slot's parent Skill or from every Skill listed in `skills`.

For a provider `P`, actor grant `G`, and actor slot `A`:

- The actor remains `OwnedActorKey { provider: P, slot: A }`.
- Its entered provider context is `P.G`.
- An actor-owned ability grant `H` targeting skill slot `S` produces `GeneratedSkillKey { provider: P.G, slot: S }`.
- Entering that ability uses provider `P.G.H`, retains the same current actor, and exposes only `S.outputs` plus the target Skill's own declarations.

Every supplied Skill needs positive membership in the exact actor slot's potential `skills` set. Partial membership is not permission to supply an unlisted definition. Potential members without a corresponding supply and activation producer retain an explicit gap. An actor definition reused in several slots must satisfy each slot's declared restrictions; no runtime filtering silently drops abilities.

Keep existing actor-slot rule owners. For example, Sniper's slot-owned baseline program can continue deriving stats on its exact actor. Actor-definition programs add the supply rules at the entered actor provider. Both run under the existing single-producer checks; moving baseline programs into the actor definition would be a separate, explicit data revision.

## Concrete Twister and Sniper paths

The names below refer to the persisted addresses in [ids.json](../data/owned/poe2/3887ae68/ids.json); production conversion must use injected IDs. New actor/supply IDs must be allocated explicitly, not inferred from these labels.

Twister already has the necessary topology:

```text
SkillUse U
  -> primary grant 0010, supplying slot 000f / Skill 000b
  -> selected output 000e, actor Player
```

Here and below the numeric suffix abbreviates `def.000000000000....`. The selected original 2 occurrence is skill set 6, group 8, Gem 1, level 19 / quality 20. Its generated skill key uses the root provider and slot `000f`; its action provider includes grant `0010`. This proposal does not change that path.

Sniper's existing path is:

```text
SkillUse U
  -> primary grant 0017, supplying slot 0016 / summoning Skill 0012
  -> population grant 0020, producing actor slot 001f
```

Let `P = U.[0017]`. The actor is `{ provider: P, slot: 001f }`; the actor provider is `P.[0020]`. Original 5 selects skill set 4, group 3, Gem 1, level 20 / quality 0. Its source minion/action selection is `RaisedSkeletonSniper` / `MinionMeleeBow` (Basic Attack). Those source tokens belong only to Import correspondence.

The new actor definition declares separate ability slots and activation grants for Basic Attack Skill `0021` and Gas Arrow Skill `0024`. Basic Attack's generated key is `{ provider: P.[0020], slot: basic_attack_supply }`. Its action provider is `P.[0020, basic_attack_grant]`, with existing output `0022` and `DeclaredActorRole::ProviderActor`. Gas Arrow uses its own slot/grant and output `0025`.

The existing output-only selector `P.[0020] / output 0022` remains a different address from the new supplied ability selector. Do not silently redirect it. Import correspondence can publish a reviewed successor selector when the new release supplies the ability. Earlier selections continue to explain their unresolved activation.

The missing `CommandSkeletalSniperPlayer` catalog references stay missing and Partial. They neither prevent representing the independently known Basic Attack path nor authorize inventing a command ability. Full-build coverage must continue to report that gap.

## Activation, inputs, choices, and nested summons

Reuse the existing effects `ActivateGrant` and `ProjectSkillParameter` for actor-definition owners. Extend their ownership validation to the new declaration owner; do not introduce a second interpreter or an actor-specific expression language.

An ability exists conditionally at its exact generated key. The planner checks the supplying actor's activation, every ancestor grant, this ability's activation producer, and required projected inputs. Known false activation yields a known inactive/unavailable ability, not zero damage. Missing producers or demanded values remain unresolved. Wrong owner, incompatible types/units, an impossible declared path, or competing producers are invalid. A failed gameplay requirement remains distinct from invalid schema and unavailable input.

Physical Gem level, generated summoning-Skill level, actor level, and actor-ability level are separate facts. Existing Sniper rules map summoning level 20 to actor level 40; that does not automatically make its Basic Attack level 20 or 40. Actor-definition rules must read the exact current actor's projected stats and supply each ability input through an authored recipe backed by source evidence. Character-level reads remain explicitly player character reads. No implicit parent-level, quality, or support inheritance is introduced.

### Reviewed ability input semantics

The pinned `calcs.createMinionSkills` in
[CalcActiveSkill.lua](../vendor/path-of-building-poe2/src/Modules/CalcActiveSkill.lua)
initializes child ability level to 1 and quality to 0. For an effect with multiple contiguous
level rows, it selects the last row reached before a level requirement exceeds the minion's
level. This is distinct from the actor level used by stat interpolation.

Sniper's Basic Attack and Gas Arrow each have one effect-level row, so their ability level
is 1 even when the summoned actor is level 40. Storm Mage's Arc provides an independent
contrast: it also has one effect-level row, but four stat-set level rows. The number of
stat-set rows cannot determine its ability level. `buildActiveSkillModList` separately
sets the effect's actor level; [CalcTools.lua](../vendor/path-of-building-poe2/src/Modules/CalcTools.lua)
uses that input for interpolation. A positive summoning quality must not be inherited
without an explicit reviewed projection.

The optional Rust [reference test](../crates/poe-optimizer-pob/tests/owned_actor_ability_inputs.rs)
executes unchanged complete source functions in both JIT modes. Its untouched original05
records physical level 20, effective summoning level 22, actor level 44 and child ability
level 1 / quality 0. Separately labelled Sniper and Storm Mage probes cover actor levels
1/20/40/100 and parent quality 0/20. They preserve the four distinct inputs and show Arc's
raw spell stats changing with actor level while its effect level remains 1. The probes do
not rebuild the actor's weapon/defence baseline after changing actor level, so they do not
establish final damage or whole-build numerical parity.

These facts refine the planned input recipes, not the Core model. Existing literal,
finite lookup and `ProjectSkillParameter` operations can express the reviewed projections;
no new arithmetic opcode is implied. The test supplies reference evidence for the named
actor-supply consumer; it does not implement native ability supply or activation.

Use existing choice scopes:

- Actor-provider choices use `ChoiceOwner::Provider(P.G)` and the actor definition's declared slots.
- Ability choices use the exact generated `SkillTarget` or its entered provider according to the slot's declared scope.
- Action choices use the full `ActionSelection` and are read by ActionOutput-owned programs.

An input admitted in one scope is not a fallback for another. `ProviderRole` continues to describe the authored root (SkillUse, ItemModifier, etc.); current actor ownership is a separate property. Initially actor definitions need no new authored parameter collection: actor facts are typed stats and generated ability parameters are explicit projections. Require actor-definition direct parameter/socket collections to be Complete-empty in the first supported version, rather than accepting inputs with no consumer.

Nested summons use the same pattern: an ability Skill or actor definition explicitly declares a child ActorSlot and grant. The child's `OwnedActorKey.provider` is the immediate supplying provider, whose path retains every ancestor. Actor-relative reads then refer to the child; projections to the child come from its exact supplying owner. Sibling actors, player stats, and remote ancestors are never inherited by registry-owner lookup. Cross-actor modifiers and support/payload propagation require their own explicit relationships and are outside this decision.

## Duplicates, bounds, and compatibility

Retain the rule that two grants into the same parent plus SkillGrant slot are ambiguous. Do not choose one, OR them implicitly, or create two skill instances. Distinct SkillGrant slots naming the same Skill are distinct occurrences; distinct summoner roots remain distinct even with identical data. Apply the same uniqueness check to alternate paths producing one actor slot, before descendants can duplicate actor-level effects.

Build immutable indexes for definitions, grants, and supplying occurrences. Charge work before cloning paths or expanding children. Bound declaration rows/bytes, potential edges, provider/actor/ability counts, grant depth, rule dependencies, and emitted diagnostics. Traverse potential topology deterministically, independent of thread or source-row order. Reject reachable recursive supply cycles as unsupported in this finite version, including inactive cycles; dynamic recursion requires an explicit finite model later. Resource exhaustion must be an error, never truncated topology claimed as complete.

Use a new schema semantics/version and update the affected rule-operation/plan identity contracts. Old packages must reproduce their prior bytes, identities, bindings, and unresolved actor behavior. The new actor definition/slots receive new registry allocations; the existing actor slot, Skill IDs, output IDs, and authored occurrences are retained. Adding the actor-slot binding and changing supplied action paths belongs in an explicit full-release revision, with all dependent artifacts rebound. Monotonic successor validation must not be weakened to accommodate the change.

This extension does not alter the full-request finalization gate, metric coverage rules, or global contribution completeness. The five originals remain incomplete until their other Pending inputs and rules are handled.

## Alternatives and trade-offs

| Option | Benefit | Cost / reason not preferred |
|---|---|---|
| **Owned ActorDefinition plus existing grant traversal** | Reuses occurrence identity, typed grants, parameter projection, and choice scopes; actor templates can be shared without sharing instances. | Adds a definition/slot-owner kind and schema version; requires coordinated exhaustive-match and validation updates. Recommended for explicit ownership. |
| ActorSlot exposes grants owned by the parent Skill | Fewer new registry kinds. | Parent and child share one declaration owner; current direct-declaration validation and rule ownership would need exceptions. Risks exposing child grants on the summoner and couples reusable actor behavior to each summoning skill. |
| ActorSlot becomes a recursive slot declaration owner | Exact ownership without a standalone actor definition. | Recursively nests declared-slot addresses, expands ownership/serialization/resource validation, and repeats actor declarations per summoner. More complex than a reusable definition with unchanged finite occurrence paths. |
| New `ProviderRoot::Actor` and separately allocated ability roots | Direct actor addressing. | Creates a second ancestry/activation mechanism, requires root lifecycle and parent binding, and can lose source/summoner provenance. Existing paths already express this ancestry. |
| Activate every member of `ActorSlotSchema.skills` | Small implementation. | Confuses possibility with supply; cannot distinguish multiple occurrences, required inputs, conditions, or nested actors. Reject. |

## Bounded implementation and acceptance tests

If accepted, implement the shared contract before authoring a Sniper-only numerical adapter:

1. Add the actor definition and versioned slot binding in Core/Data, explicit declaration validation, and bounded provider exposure. Update rule owner ports and plan discovery to reuse supply/activation handling. Preserve the old reader path.
2. Author one new Sniper actor definition with two distinct ability supplies, finite activation/input recipes, and unchanged existing actor baseline/output IDs. Establish the real ability-level projection independently; do not infer it from source UI indices.
3. Publish a reviewed full release and update Import action correspondence to the new explicit ability path. Preserve all 110 query IDs/order/metric selectors; unresolved metrics remain unresolved.

Rust tests must cover two summoners sharing one actor definition; two ability slots naming one Skill; inactive and missing activation; missing/wrong-unit projected inputs; parent versus actor versus ability choice isolation; nested summons and sibling access rejection; alternate-path ambiguity; wrong actor/output combinations; cycles and tightened pre-expansion limits; legacy byte/identity reproduction; and deterministic serial/parallel plans. Include a small synthetic second actor family to prove the production path has no Sniper dispatch. Optional PoB tests validate the source correspondence and ability-input recipes separately from full metric parity.

## Work that can proceed before approval

Import action correspondence can use the current `ActionSelectionDraft`, `OwnedActorKeyDraft`, `ProviderKeyDraft`, and typed output slots now. It can prepare the selected Twister action, Sniper summoning action, Sniper actor, and its Basic Attack output without changing Core. Bind these selections to source occurrences and injected topology data, retain all 110 queries, and report current supply/metric/closure gaps. Do not add fabricated SkillUses, clear Pending collections, or label output selection as activation.

The concrete current blockers are in [actor schema](../crates/poe-optimizer-core/src/owned_schema.rs), [provider binding](../crates/poe-optimizer-core/src/owned_binding/selectors.rs), [rule owner validation](../crates/poe-optimizer-engine/src/owned_rules/compile.rs), and [plan discovery/gates](../crates/poe-optimizer-engine/src/owned_plan/compile.rs). The existing [definition contract](owned-definition-package.md) and [real component tests](../crates/poe-optimizer-import/tests/owned_recipe_real.rs) remain authoritative for implemented behavior until this proposal is accepted and implemented.
