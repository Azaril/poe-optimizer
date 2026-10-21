# Decision: separate modifier ordering contracts and preserve canonical numeric inputs

Status: accepted by the user on 2026-10-20; implemented and validated on 2026-10-21. The initial evidence came from main `4eb2a112`, Charm package `e85a165738460fa2bc37f3eda92a0732c26771192e4faf999827d8f29864a9f6`, and original-source controls in `runs/modifier-transform-source-02Oawm`. The migration starts from main `c7e53685` and the validated Charm scalability package `b8851ea50c9d5fd97e1695e536c89dba185f136d51d6e77da31bd22f8f4a67a9`. The current format was changed in place without a compatibility path. [Validation receipt](evidence/modifier-order-fixture-migration/validation.json); outstanding producer-data evidence remains listed below.

## The demonstrated problem

The source test passed 31 cases in four independent fresh VMs, with JIT on/off, and preserved the original functions. Two distinct orders are observable:

- Starting with a Charm component of 4, an Add 50% producer followed by a doubled producer yields 12. Reversing them yields 10.
- An explicit Add producer discovered before a later tagged implicit doubled producer still yields 12. Grouping the records implicit-before-explicit would yield 10.
- Later sorting of explicit records also preserves the earlier producer discovery order.

Before this migration, native `ItemRecord.modifier_order` was used for both magnitude-transform folding and ordered numeric-contribution reduction. The Import contribution census constructs implicit ordinals followed by explicit ordinals. These source observations prove that this one field cannot generally describe both required orders. They do not prove that every source contribution family has correct ordering today, or that PoB's ordering is universally the live game's intended arithmetic.

The encoding distinction is also observed: ordinary fixed source text remains 4 while authored fixed or ranged input produces 6 under the same 50% producer. Source comments state that ordinary/trade data may already be scaled. These cases prove that importing every spelling as raw and applying every producer would be wrong. They do not prove an inverse from a rendered value to an original raw roll.

## Accepted native contract

Keep one item record and one set of modifier occurrences. Replace the ambiguous single order with two explicitly named semantic orders, for example:

```rust
struct ItemModifierOrders {
    contributions: Vec<ModifierInstanceId>,
    numeric_transforms: Vec<ModifierInstanceId>,
}

struct ItemRecord {
    // Existing item identity, template, parameters, quality and modifiers.
    modifier_orders: ItemModifierOrders,
}
```

Each order is a complete permutation of that item's exact modifier IDs. Neither storage order nor occurrence IDs are a fallback. The draft has independent pending/known states for these orders, so proving one does not certify the other. Duplicate, foreign, or missing IDs are rejected independently.

The ordered-contribution binder uses `contributions`. `ProjectModifierTransform` binding uses `numeric_transforms`, followed by its existing per-producer step. All magnitude channels currently share the same source producer ordering; there is no evidence yet requiring a distinct order per stat/channel. Keep that choice narrow rather than add an extensible string-keyed order registry now. The two domains are mechanics concepts, not source line numbers, categories, display positions or UI flags.

This preserves exact occurrences, duplicate producers, current applicability rules, noncommutative arithmetic, closure failures, and worker-local scratch. Order resolution remains a cold preparation operation. Evaluation workers execute the same compiled graph; no new interpreter or backend is needed.

Alternative: keep a single sequence and accept only imported layouts where both orders provably agree. This is a valid temporary admission restriction, but it cannot represent the demonstrated inputs and gives the optimizer an unnecessary structural limit. It should not be the end-state model. A general map of named order domains is another option, but adds validation and policy surface without evidence of a third required order.

## Accepted Import contract

Keep native roll slots canonical and source-free. Do not add `crafted`, `advancedCopy`, `already_baked`, or source-version switches to native records or evaluator rules. Extend the existing item attribution and normalization accounting to establish these facts before it emits canonical values and order identities:

1. The exact physical modifier membership and correspondence between source members and emitted occurrence IDs.
2. The contribution sequence and the numeric-transform sequence, with separate source evidence. In the current PoB adapter this includes discovery order before display/category sorting. Only IDs leave Import.
3. Whether each numeric input is raw, already transformed, or unresolved. This classification and its proof stay in Import reports/sidecars. A source spelling can establish raw authority only inside its checked full-item lifecycle, not from an isolated line or the presence of an arbitrary header.
4. Every producer's participation and every unsupported or ambiguous source member. Unknown producers, activation or input encoding continue to block completeness.

`ItemRangeAttribution` already owns raw text, source positions, range writes, flags, checked context Options and complete-layout proof. `NormalizedItemText` already carries attribution, physical input evidence, exact emitted IDs and issues. Extend these existing structures and their source accounting; do not add a parallel importer or a second build model. The builder already captures `(line, emission) -> ModifierInstanceId` in `normalize_item`; use that checked mapping to construct both semantic orders.

For already-transformed values, the first production policy should require either a proven identity transform or a unique, exact reconstruction of the canonical input. Otherwise leave the raw slot pending with an explicit encoding obligation. Division by a magnitude is not a generally valid inverse after formatting, truncation, corruption or multiple transforms. Do not silently omit a producer or disable its native effect to make imported totals match. Such a shortcut would break subsequent item edits and optimization.

Longer-term direct support for resolved values whose raw rolls are irrecoverable may need a separate, generic partial-knowledge contract. That is not necessary to establish the current raw-input path and should be a deliberate future model decision. The current proposal does not invent a second native value path merely to bypass admission.

## Subsequent producer-data authoring

Use the existing ProjectModifierTransform instructions. Percentage producer amounts emit dimensionless Add steps through `PercentAsFactor`; doubled definitions emit Multiply 2. Preserve the producer's pre-transform amount separately from its own final transformed value. Do not introduce feedback by reading the producer's final `253d` output or silently bind a pre-transform scalar to the numeric compiler's final-factor field.

The expanded 56-case source suite passes in four fresh VMs, joining 47 scalability rows and 15 upstream modifier IDs to live authenticated source tables. Its 45 numeric template rows mark producer strength unscaled. Nonneutral catalyst/corruption controls confirm that strength is prepared independently. Use existing `ModifierValueScaling::Unscaled`, then `PercentAsFactor` and Add; no new compiler API is needed for those templates. Doubled forms remain literal Multiply 2. This does not extend to arbitrary noncatalog spelling, whose metadata fallback has different observed arithmetic. The tripled catalog form creates no producer in the tested source path and remains a discrepancy. These are source controls, not a native production conversion.

Publish finite authored recipient predicates across matching canonical families, rather than special-casing Charm. The current package inventory contains 47 canonical consumers with explicit unscalable inputs; 10 have category input slots, and only the Charm family has a charm-property slot. This consumer inventory is distinct from the 47 producer metadata rows above. Add missing required properties/categories through existing data/Import projection seams where their authority is established. Missing predicate facts remain unresolved, not false. Retain skill-grant exclusions and source participation decisions as independently reviewed obligations.

## Migration and validation

- Change the current format in place: core record/draft validation, canonical identity, the two plan binders, Import construction/accounting, authoring fixtures and documentation. Regenerate current artifacts and imports. Do not preserve the ambiguous old wire meaning.
- Replace tests that assume one sequence with explicit identical-order fixtures where that is the intended case. Add one meaningful case where the two sequences differ and assert both the transform result and contribution result. Reversing record storage must affect neither result.
- Add malformed/partial order and missing encoding controls. Prove exact source-to-owned occurrence correspondence and preserve source diagnostics for unsupported variants or baked inputs.
- Validate the original Fine Belt through the complete current pipeline. It already has `Crafted: true` and a ranged Charm line; today's fresh physical proof admits it. This work should not classify it as an unsupported ordinary fixed item.
- Reimport all five originals, preserve all 110 query rows, and update the living implementation resume point. The 31 source controls and native component controls are evidence for these contracts, not complete-build parity or closure authority.

## Decisions versus further evidence

**Accepted:** two explicit native ordering domains and Import-only encoding authority, with raw canonical native slots retained. The source controls establish that the existing single-order assumption is insufficient. Approval authorizes the structural migration; it does not close the remaining source evidence or production coverage obligations below.

**Still open implementation evidence:** exact contribution ordering for each non-Charm family; full producer syntax/category/tag coverage; pre-transform amount precision/catalyst/corruption domains; mathematically justified recovery of already-transformed raw values; how to classify the observed legacy inactive-producer anomaly. That anomaly should remain a source discrepancy until reviewed, not become a native rule allowing inactive modifiers to affect other occurrences.

The October 21 expansion passes 59 cases in four fresh JIT-on/off replays
(`runs/modifier-transform-source-YD4AvL`). With authored raw Charm capacity 4,
Add 50% followed by Add -50% leaves fixed text at 6, while the equivalent ranged
text returns to 4. Ranged raw 4 with recipient corruption 1.5 and magnitude 1.5
produces 9. These observations confirm distinct source reparsing paths; they do
not establish intended game behavior or new Import admission authority. Retain
the fixed/ranged discrepancy in the source-boundary investigation. Native rules
continue to consume canonical inputs and the final ordered factor without
copying the source's intermediate parsed state.
