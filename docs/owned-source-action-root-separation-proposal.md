# Proposal: separate source selectors from skill supply

**Status:** Proposed; not accepted or implemented.
**Date:** 2026-10-08.
**Decider:** Project owner.
**Scope:** Owned Import only. No new native occurrence, evaluator or source-language runtime.

## Context

The Original05 accounting audit found eight archived source rows whose only
current owner is configuration obligation `01f2`. Four Skill/Gem pairs describe
the item-granted Warrior from saved Item24, Rattling Sceptre `1fe3`:

| Saved SkillSet ID | Source ordinals | Existing generated / usage / support obligations |
| --- | --- | --- |
| 3 | 185 / 186 | `067b` / `04f0` / `01d9` |
| 5 | 246 / 247 | `067c` / `052b` / `01e3` |
| 6 | 351 / 352 | `067d` / `055a` / `01e8` |
| 1 | 418 / 419 | `067e` / `059e` / `01ed` |

The source reference Gem is `091a`, primary Skill `0329`. Pinned
`Data/Skills/act_int.lua` declares `SummonSkeletalWarriorsPlayer` with
`fromItem=true` and singleton `RaisedSkeletonWarriors`. `Data/Minions.lua`
declares its one child, `MinionMeleeStep`. The existing owned child Skill `0292`,
output `32a0` and stat set `32a1` can be reused; they do not prove the missing
Warrior supply or source selectors. Saved raw level 11 and quality 0 remain
source observations, not new native inputs.

The intended accounting result is to reduce `01f2`'s source links from 44 to 36
while preserving each real unresolved owner. It would not complete `01f2`,
retire any of the five selected input issues, enable Warrior or change the
selected Sniper request. No such reduction has been published.

## Current boundary

[`SourceActionCorrespondenceInput`](../crates/poe-optimizer-import/src/owned_source_actions.rs)
combines source selector interpretation with root-supply authority. Its physical
variant requires a real Gem primary supply and entering grant; its manual
variant requires a directly selectable Skill and admitted manual source.
[`minion::compile`](../crates/poe-optimizer-import/src/owned_source_actions/minion.rs)
correctly validates those relations before admitting the Actor/action path.

[`GeneratedSkillInputRule`](../crates/poe-optimizer-import/src/owned_normalize/generated_skill_inputs.rs)
already validates an exact Item/Tree provider and raw-input binding, but has no
injected minion/action/index/absence correspondence. Therefore neither existing
source-selector variant describes an item-only Warrior honestly. Adding a Gem
grant, marking Warrior directly selectable, or skipping either validation would
invent authority. A private reuse experiment was removed after this preflight;
no importer, schema or data change remains from it.

This also exposes a general separation issue already noted in the
[generality review](build-generality-review.md): preserving source fields should
not require manufacturing a native supplier or succeeding at a reference query.

## Recommended decision

Factor the current source-action contract into two independently checked parts:

1. **Source selector correspondence:** authenticate source/catalogue identity,
   primary Skill, singleton Actor topology, child actions, part/mode/stat-set
   mappings, index codecs and explicit missing-value rules. Compile this once
   with the same bounds and exact declared-slot checks for every caller.
2. **Supply binding:** authenticate the actual physical Gem, manually authored
   Skill, or generated Item/Tree supply using its existing authority checks.
   A catalogue Gem identifier may identify source data without asserting a
   physical Gem occurrence. A successful selected output still needs an exact
   resolved supplier and the full owned target path.

The current catalogue's `Physical` materialization classification alone must not
be promoted into that supplier proof; item-only source metadata can still carry
a catalogue Gem identity.

Both parts must agree on the primary Skill and source identities. Do not replace
the existing checked roots with an unchecked generic provider key. For archived
deferral, require all three actual same-preset Pending owners and complete
source-field accounting. Deferral emits no resolved provider, raw input, count,
query target or numerical coverage. Unknown or ambiguous fields retain their
current obligations.

Keep one compiled minion/action implementation. Replace the current development
format and rebuild affected artifacts; retain no compatibility branch solely to
replay the earlier interpretation. Source spellings and MAIN/CALCS selectors
remain in Import. Native Core/Data/Engine continue to consume the existing
source-independent occurrence and action contracts.

## Alternatives and consequences

| Option | Benefit | Cost and limitation |
| --- | --- | --- |
| Separate selectors and supply binding — recommended | Matches the distinct responsibilities and shares one checked selector implementation across authored and generated sources. | Requires a deliberate Import format migration and rebinding current policies. |
| Add a generated correspondence variant | Can express this case with explicit generated-provider checks. | Extends the current coupling and risks duplicating the selector fields/validation for every root kind. Shared compiled logic would still be required. |
| Defer this work | Preserves current conservative behavior. | The eight fallback links remain unresolved; a different owner-proof path would still need review. |

This decision is separate from native Actor/reward and Skill contribution-query
authority. Those remain the closest numerical path for Original05 and should
not be delayed by this source-contract work.

## Implementation gates after approval

- Specify both parts and their agreement/invalidation rules, migrate physical
  and Direct callers, and remove obsolete variants rather than keeping parallel
  import modes.
- Author genuine Item-to-Warrior-to-Actor topology and reuse the existing child
  action. Authenticate constructed source identity/selectors; do not create an
  obtainable physical Gem merely because PoB retains Gem metadata.
- Keep item-grant numerical transport and successful generated inputs separate
  from archived accounting. Prove that the latter preserves the canonical
  draft, local identities, selected queries and all existing obligations except
  the eight specifically justified fallback links.
- Test repeated/ambiguous sources, missing/Complete/foreign-preset obligations,
  mismatched primary definitions, malformed/unsupported selectors, declared
  absent-value rules, unsupported fields and bounded work. Test selected live
  output refusal independently from archived deferral.
- Reimport all five unchanged originals, preserve all 110 queries, and confirm
  the target 44-to-36 link change without claiming a complete input inventory
  or native build. Do not make runtime behavior depend on build IDs or names.
