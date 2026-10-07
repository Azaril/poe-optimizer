# Generated source ownership and final-input assembly

Status: accepted and implemented in the native contract on 2026-10-07. Real
Sand/Firebolt preparation data and complete-build coverage remain separate gates.
Current integration: `runs/owned-generated-participation-02/package`.

## The actual blocker

Original05 has tree-generated Sand Djinn and item-generated Firebolt, plus a
separate manual Sand Djinn. Their raw inputs already have exact owners and the
saved usage importer can bind each occurrence independently. The existing
participation policy can therefore be attached without new model code.

That does not make these Skills ready for native execution. Their parameter
inventories still contain raw inputs only and remain Partial. Source observations
show why raw cannot mean final: Original05's generated Sand starts at level 1
and prepares at level 3; Original01's starts at 1 and prepares at 12. Original04's
Firebolt changes from raw level 17 / quality 0 to prepared level 26 / quality 21.
These examples identify the missing assembly stage, not universal formulas.

Before this extension, the shared source-property layer discovered only authored
SkillUses. Generated effect endpoints were children of those owners and could
not themselves own collected properties. The extension removes that limitation;
it does not supply missing game rules. Sand's source `noSupports` setting does
not eliminate external level/quality properties.

## Recommended ownership

Extend the existing relation to exact generated Skill occurrences. Continue
using the existing typed `SkillTarget`, provider path, supply slot and immutable
occurrence graph as identity. Do not manufacture an authored SkillUse, physical
Gem, second source entity or another copy of raw inputs.

Relations must explicitly declare which occurrence forms and supplying
declarations they admit. A relation for generated Sand must not silently bind
the manual Sand with the same definition. Repeated providers and independent
equipment/loadout occurrences remain separate. Discovery follows validated
supplies, regardless of which Action is queried or whether supports are present.
The Player/scenario context remains bounded; this does not silently authorize
owned-minion source contexts.

Generated applicability names a supply declaration in the release, not a
concrete allocation, item instance or saved preset. Cold binding obtains those
concrete identities from the candidate. Data-level uniqueness includes this
applicability; overlapping relations still fail for a concrete source owner.

An effect endpoint may refer to the source's own concrete Skill or to an exact
declared descendant. A physical Gem owner still has no direct Skill effect of
its own. Eligible effects and admitted support positions keep their existing
explicit inventories and once-per-source collection rules. The same source
position seen through two effects contributes once; independent retained
positions remain distinct.

## Recommended final-input producer

Keep property ownership separate from write authority. A generated source owns
its aggregate properties, but its **exact supplying provider** projects final
parameters through the existing `ProjectSkillParameter` operation. This reuses
the authority already used for raw tree/item supply; it does not allow a Skill
to write arbitrary parameters on itself.

The relation needs an explicit assembly binding to either its input owner or
that exact supplying provider, with owner-qualified program identity. Static
validation checks the declaration; cold binding checks the concrete provider
and supplied target. A supplying-provider assembly invocation may project only
into this relation's exact supplied Skill. It cannot target siblings, another
item copy, a different allocation or an arbitrary ancestor. No provider search
by display name or a nearest-parent heuristic is allowed.

For a generated target, use its declaring provider (`GeneratedSkillKey.provider`)
and exact supply slot. Do not use the entered child provider, which would shift
projection down another level. Infer the assembly definition owner from that
slot's declaration and require every projection to name that same supply.

The assembly program retains its natural provider context: Actor for a passive
allocation and EquipmentUse for an item modifier. Its property reads are sealed
to the separately bound source Skill. This resembles the existing external
producer/source-owner separation, but grants an explicit final-assembly role
and checked projection authority. Existing physical Gem child projections and
Direct effective Skill stats retain their meaning.

Raw and final slots must remain distinct. Author their full required-input
inventory and producer coverage before publishing readiness. Required final
values are deferred to Execution, never made optional or replaced with raw
values. Preparation dependencies must not demand their own final output.
Missing producers, unavailable providers, competing writers and cycles remain
errors or unresolved results, not defaults. A false participation preference
gates execution after mechanical preparation; it does not erase the source.

## Alternatives and consequences

| Approach | Benefit | Cost / limitation |
| --- | --- | --- |
| Exact Skill occurrence ownership plus exact-provider projection — recommended | Reuses raw-input identity, source collection and existing projection authority; handles item/tree/manual contrasts without a duplicate build model. | Extends relation discovery, assembly bindings, validation and readiness dependency checks. |
| Put aggregation on item/passive provider records | Final projection is naturally local to the provider. | Couples source-specific quality and support membership to providers which can supply multiple independent effects; risks duplicate aggregation and a second ownership model. |
| Let generated source assembly write its own final parameters | Keeps aggregation and assembly in one context. | Introduces new self-write authority and associated cycle/producer rules; conflicts with the currently accepted prohibition until explicitly redesigned. |

The recommendation prefers a single occurrence model and narrow producer
authority over the smallest enum addition. Its exact DTO shape should follow
implementation review; old development formats need not remain compatible.
Rebuild affected data and result identities instead of maintaining old/new
execution branches.

## Accepted contract shape

Update the current receiving V3 contract in place. Occurrence applicability is
explicitly `AuthoredSkillUse {}` or `GeneratedSkill { skill_supply }`. Rename
the former `DirectOwner` endpoint to `OwnerSkill`; a physical Gem container
still cannot use it. Assembly entries carry a program key and either
`InputOwner` or `ExactSupplyingProvider`. Infer their definition owner from
that binding, so an independent owner field cannot disagree with the supply.

For generated input-owner programs, existing Skill facts and declared child
projections retain their scope; this adds no self-parameter write operation.
Exact supplying-provider assembly projects only to the relation's own generated
Skill. Missing natural provider context is an error; nested equipment providers
are not resolved by guessing an ancestor. The first supported equipment binding
uses the existing exact EquipmentUse/ItemModifier root context.

Manual/generated uses of the same Skill must share their effective calculation
where their mechanics agree. Existing `InputOwner` assembly can derive common
prepared Skill stats for both occurrence forms, with an exact-provider projection
bridge only where generated consumers require final parameters. This does not
authorize importing prepared literals as manual raw inputs. Source-specific
level/quality domain validation and complete producer/consumer inventories still
belong in authored rules. A shared stat design for Sand is within this contract;
a universal replacement of all existing final parameters would be a separate
design decision.

The canonical receiving input includes these fields, and its identity feeds
effect and metric plans and evaluation receipts. Rebuilding a source-property
relation therefore changes affected cache/result identities. Old wire forms
are rejected, not interpreted under new semantics. Empty source-relation
inventories keep their existing behavior. Current Partial releases without an
evaluation bundle do not embed this relation contract.

## Implementation and acceptance gates

Gates 1–3 are implemented. Validation covers three Core wire tests, 63 Data
receiving/stage tests and 48 Engine tests including ten new generated-owner
cases. Current-release numerical components pass: Ice Nova eight, Offering
eight and Sniper six. The all-five participation replay reproduces its prior
identity and issue inventories. Gates 4 and 6 remain real data/numerical work;
gate 5's participation transport is published, but readiness remains withheld.

1. Generalize the existing relation's owner discovery and explicit applicability.
   Bind generated owners through the shared resolver and structural supply map.
   Preserve authored relations without widening their admitted forms.
2. Add checked assembly producer bindings and exact output restrictions. Use
   existing parameter projection, source-property reads, stages and worker
   scratch. No Lua runtime or source names belong in native execution.
3. Test repeated tree/item providers, manual and generated uses of the same
   Skill, inactive loadouts, missing/ambiguous sources, duplicate relation claims,
   foreign/sibling writes, missing producers and dependency cycles. Verify that
   support collection and property application remain once per source.
4. Author real Sand/Firebolt external-property and final-input rules with source
   evidence, full required-input inventories and independent final-domain checks.
   Raw transport and finite fixture tests do not satisfy this gate.
5. Publish participation for every admitted form of a Skill before enabling a
   definition-wide readiness gate. Manual Sand requires its own policy binding;
   unresolved Original04 Firebolt must remain unresolved. Preserve all presets,
   110 queries and unrelated input obligations in the five-build replay.
6. Compare the actual prepared values and requested execution states against the
   pinned source, preserving its selected-group preview override as diagnostics.
   Require serial/Rayon and reused-scratch determinism. Complete-build parity
   remains a separate gate, currently 0/5.

Saved usage transport has been published independently through the existing
policy. Do not close parameter inventories or publish generated readiness to
conceal missing real preparation rules and contributor coverage.
