# Checked contributions from exact Skills to Actors

**Status:** Proposed; requires the project owner's decision. No runtime change or Graft publication is authorized by this document.
**Date:** 2026-10-24.
**Scope:** Preserve the exact Skill source of a direct contribution to the Player, including repeated occurrences, in the existing checked contribution graph.

## Concrete requirement

The pinned Sniper definition supplies a Graft magnitude of 15. When the Player's Umbral Well flag is true and that Skill's resolved magnitude is positive, PoB suppresses damage on that Skill's own summoned actor and adds `magnitude × requested count` increased Projectile Speed to the Player. Each active Skill occurrence adds its own contribution. This is a direct additive path; the `GlobalEffect=Buff` tag does not send it through the Offering maximum-selection path.

The actual activation source is tree allocation 26085's Abyssal Lich view, Umbral Well. PassiveSpec replaces its mechanical fields while retaining the allocation's identity and topology. The current owned Passive `0ee8`, guarded by the actual Ascendancy `0a29`, can therefore supply the activation flag. It must not supply that flag for the base Lich view. These IDs are existing data bindings, not runtime dispatch keys.

The source investigation and unpublished draft are in `runs/revival-review/graft-authoring-plan.md` and `runs/revival-review/graft-draft/`. They retain the distinction between intrinsic magnitude, activation, requested participation/count, exact child suppression, incoming-source completeness and complete-build readiness.

### Retained source anchors

All anchors below refer to revision `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`, authenticated by the tracked PoB source manifest. They preserve the actionable findings independently of the ignored draft.

- `src/Data/Skills/act_int.lua:18677–18799` supplies the complete Sniper definition and magnitude 15; `src/Data/SkillStatMap.lua:2986–2988` maps it to `UmbralWellBuffValue BASE`. `src/Data/Minions.lua:83–120` establishes admission through `ActiveSkeletonLimit`, which is distinct from requested count.
- `src/TreeData/0_5/tree.lua:67863–67905` contains allocation 26085 and option 41162. The class/Ascendancy binding is at 611–624. `PassiveTree.lua:448–595`, `ModParser.lua:2172–2179,3186–3188` and `PassiveSpec.lua:1483–1494,2047–2063` establish option parsing, Boolean production and identity-preserving replacement.
- `src/Modules/CalcPerform.lua:1351–1392` contains the conditional child suppression and direct additive Player write. `CalcDefence.lua:149–174` supplies the requested-count behavior. `ModStore.NewMod`, `ModDB.AddMod` and `ModDB.SumInternal` preserve separate records; the GlobalEffect tag does not imply maximum stacking.
- `src/Modules/CalcOffence.lua:2425–2429` combines damage-prohibition causes. Its downstream hit, damage-over-time and ailment consumers require integration; projecting one cause alone does not finish damage suppression.

Current exact bindings are Gem `0011` → SkillGrant `0016` → Skill `0012`, with own ActorSlot `001f` and child Actor `3091`. Requested count uses UsagePolicy `326a` / Parameter `326b` (0..4) / Stat `326c`; participation uses UsagePolicy `332b` and Stat `332e`. These are source bindings, not runtime branches. The magnitude's foreign modifier filter currently uses PoB's selected-main-Skill configuration and still needs reconciliation. Original05's absence of allocation 26085 proves only that this particular producer is inactive, not that every possible activation source is absent.

## Current boundaries

The existing DSL can express the guarded activation, magnitude, exact own-child Boolean projection and direct `contribute(player, ...)` effect. It cannot yet admit that effect to a checked Player query:

- Data's [membership validator](../crates/poe-optimizer-data/src/owned_rules/ordered.rs) requires every `ContributionOrigin::Skill` member to contribute to `Current`. The existing source permissions otherwise already bind the exact direct or generated Skill occurrence.
- The [Engine query binder](../crates/poe-optimizer-engine/src/owned_plan/compile/ordered.rs) has no semantic numeric order between repeated occurrences of the same Skill producer. Their authored source/program/effect ranks tie. Opaque IDs, discovery order and worker scheduling must not break that tie. Build Skill tables are canonicalized by ID; table order is not an authored arithmetic order.
- The [application model](../crates/poe-optimizer-core/src/owned_rules.rs) only has `EffectStackingReduction::Maximum`. Its [compiler](../crates/poe-optimizer-engine/src/owned_plan/compile/effect_applications.rs) produces one maximum group result. It has no independent per-source delivery policy. Using it for additive Graft would change the mechanic.

Removing only the Current-recipient check would therefore be insufficient. It would leave repeated numeric sources unsupported, and would fail to state their arithmetic contract.

## Recommended decision

Reuse **ProgramEffect membership with its exact Skill origin**, admit the explicitly authored **literal Player** destination, and add an opt-in **exact quantity sum** to the existing query reductions. This is one contribution system, without a second application or aggregation path.

### Source and recipient authority

Keep `ContributionOrigin::Skill { authored, supplies }` and its existing source checks. The effect itself already declares its destination, so a second destination field is unnecessary. Admit `Current` and `Player` only; validate the concrete target against the declared destination during cold planning and late support validation.

For `Current`, preserve the exact Skill recipient. For `Player`, require the concrete `ActorKey::Player`, while retaining the source Skill's full occurrence/provider identity. The Stat must admit Actor values. This does **not** make `RuleEntity::Actor` valid in Skill context or mean “the Skill's summoner.” It grants no arbitrary related-Actor, Enemy, cross-Skill, Gem source-property or support-origin authority. A future generic summoner/recipient relation needs its own explicit contract.

Keep each repeated direct/generated Skill occurrence, each program effect and its exact supply permission. Never deduplicate equal values or definitions. A missing, crossed, ambiguous or foreign supply remains an error. Participation, loadout and readiness gates retain their existing meaning; a disabled source remains in the potential contributor census.

### Repeated numeric contributions

Add `ContributionReduction::ExactQuantitySum` as an explicit **unordered** reduction for finite same-unit Quantity `Add` and `Increase` channels, with a same-unit zero identity. Accumulate the exact mathematical sum of the input binary floating-point values and round once, using nearest with ties to even, to the existing finite quantity representation. Every zero result is positive zero, independent of input signs or order. This is not the existing ordered floating-point left fold and does not replace it.

The implementation must use bounded native arithmetic and bounded reusable worker scratch, with no allocation per contribution. Validate units, charge each candidate, preserve every source identity and reject a non-finite final result. It must produce the same result for permutations and parallel partitioning: merge unrounded accumulator states, never already-rounded subtotals. Intermediate cancellation must not overflow merely because one incidental sequence would do so. Unknown active inputs and incomplete membership still make the result unavailable; failing members keep deterministic existing failure/provenance behavior. False/inactive effects remain absence; they are not forged zero inputs.

This is a **new numerical policy**, not a claim that every PoB floating-point fold is order independent. Its initial Graft proof domain is the reviewed magnitude/count cases for which all produced terms and their sum are exactly representable. Fractional or extreme foreign modifiers require their own parity/domain evidence before their incoming-source census can close. Existing ordered reductions, explicit rounding nodes and integer reductions remain unchanged.

This addition is preferable to inventing occurrence ranks from IDs or storing a new build-wide order solely to sum a mechanic whose intended result is additive. It is also useful for future additive cross-occurrence mechanics. It must remain opt-in and independently tested.

## Alternatives

| Option | Benefit | Cost or limitation |
| --- | --- | --- |
| Exact Skill → Player membership plus exact quantity sum (recommended) | Reuses actual source/effect ownership and checked queries; repeated sources remain distinct; no new build order or application path | Adds an explicit numerical reduction; outside proven domains it can differ from an ordered PoB fold |
| Exact Skill → Player membership plus authored occurrence ordering | Reproduces an explicitly selected floating-point left-fold order | Requires a real order contract across direct, item/tree-generated and nested Skill occurrences, persistence/import/search invalidation, missing-order checks and UI decisions; current canonical table order cannot supply it |
| Independent per-source applications | Makes source/recipient delivery explicit through the existing application graph | Requires a new stacking/delivery policy and checked producer address for individual applications; still needs a numerical order or exact-sum decision; unnecessary for a direct contribution already expressible by the source program |
| Existing ExactIntegerSum over a checked quantity/quantum domain | Reuses proven integer arithmetic for an intrinsically integral mechanic | Only honest when the source establishes exact integral quantum membership and conversion is checked without rounding; current Sniper base 15 and count 0..4 fit, but unknown/fractional modifiers do not. Do not create duplicate integer percentage channels or force future modifiers into that domain to bypass the real channel |
| Keep the present boundary | No public change | Graft remains an unpublished component draft; a constant-only or single-copy-only release would not satisfy the mechanic |

Do not use Maximum, a child Actor proxy, raw unchecked contribution reads, integer stand-ins for arbitrary percentages, or a complete-empty query to avoid these choices.

## Implementation boundary after approval

1. Core: update the documented Skill destination authority; add the opt-in reduction and its typed contract. No additional source identity or recipient selector is required. Update the current contract and its identity in place, then rebuild data; no old/new compatibility mode or parser. Unrelated existing operation-version retirement remains separate debt.
2. Data: extend the exact destination check and reduction shape checks in `owned_rules/ordered.rs`; retain all source/owner/supply validation and validate raw standalone inputs too. Extend the existing `RuleReadSource::Contributions` restriction so the new reduction cannot bypass checked query membership. Preserve collection/work limits before allocation.
3. Engine: retain source identity independently of the Player target in the existing query binder: the effect invocation still belongs to its exact Skill/provider while the target is `ActorKey::Player`. Verify destination at each binding path, including late support census, in the same change. Lower the new reduction to one bounded worker operation using immutable candidate indices and local scratch. Do not alter existing ordered rank semantics.
4. Authoring: add the reduction spelling to the existing KDL query frontend and any corresponding shared reduction syntax. No new source JSON format or gameplay-specific opcode.
5. Graft data: add source-proven activation/magnitude, complete conditional own-child projection and direct Player effect; declare exact query membership with the chosen reduction. Keep current owner/query gaps and evaluation absence until the actual source census and owner execution obligations are satisfied.

The exact code change should remain small relative to introducing a new application family. Numeric implementation details and bounds need a review before coding; this document does not choose a library or permit an unbounded accumulator.

## Acceptance laws

- A valid exact Skill source contributes to Player; its self contribution still targets that exact Skill. Wrong target, Stat type/unit, owner, direct permission or generated supply fails.
- Two equal Skill definitions in distinct occurrences both survive. Counts 1 and 2 with magnitude 15 yield separate 15 and 30 contributions and a 45 subtotal. Swapping storage order, rebuilding, reusing scratch and running workers in parallel leaves results and provenance equivalent.
- Independent mathematical quantity-sum tests cover cancellation, subnormals, negative values, zero/sign rules, ties-to-even, exact and overflowing final results, permutation/partition equivalence, unit mismatch and exact/one-short budgets. Separate pinned-PoB tests establish the admitted game domain. Ordinary Sum still rejects missing semantic order; no old reduction silently changes.
- False activation skips unavailable magnitude/count reads; true activation with unknown required inputs remains unavailable. Positive magnitude with count zero still suppresses the exact child and contributes a known zero. The suppression dependency must not require count.
- The activation positive control selects allocation 26085 with Abyssal Lich. Another Ascendancy, an unselected node and a missing/ambiguous source cannot activate it. Base-view mechanics remain separate and Partial.
- Child suppression applies only to the exact supplied actor and its damage paths. An unrelated actor, another Skill occurrence and the Player remain untouched. A Graft-specific Boolean cannot falsely clear other damage-prohibition causes.
- Duplicate, missing, unread, inactive, identity-valued and late-introduced contributors are checked. Partial source domains, missing owners and whole-plan readiness remain blocking. Tests must not complete fixture owners merely to claim Original05 works.

Approval should cover both destination authority and the numerical choice. If exact summation is not accepted, select and specify a real occurrence-order model before publishing the Player contribution.
