# Proposed item modifier properties and scaling

**Status: Proposed next slice.** This document describes input and calculation seams, not
implemented tag/catalyst support. The unmodified original05 ring remains Pending, and full
original-build native completion remains **0/5**. Existing complete-request and contributor
coverage requirements are unchanged.

Convert source annotations into owned modifier properties and item inputs, then calculate
their effects against the exact item/modifier occurrence. Preserve meaningful numeric
stages without importing PoB's Item objects, mutable line lists or UI lifecycle.

## Data, input and calculation

| Layer | Responsibility |
| --- | --- |
| Offline definition conversion | Map exact property labels, catalyst identities/aliases, selectors and each numeric component's precision/scalability into schema-bound owned data. Source pins are optional provenance, not runtime dependencies. |
| Saved-build import | Preserve known labels, category/affix-side flags, authored ordinary quality, catalyst selection/amount, source value encoding and unresolved siblings. Record explicit source defaults only through reviewed policy and provenance. |
| Native resolution and calculation | Follow the actual owning item, bind applicable transforms in their declared order, and calculate effective values from owned inputs. Missing facts or incomplete transform membership withhold the affected result. |

Modifier property labels such as `cold_resistance`, `elemental` and `cold` are distinct
identities. They are not actor conditions or interchangeable strings. Existing legacy
`ModifierTag` condition machinery must not become their model. Unknown properties remain
unresolved; an unrecognized label cannot silently become false or be discarded.

The pinned [Item implementation](../vendor/path-of-building-poe2/src/Classes/Item.lua)
provides the reference for label/category handling, catalysts, magnitude selectors and
source encoding. [ItemTools](../vendor/path-of-building-poe2/src/Modules/ItemTools.lua),
[Common numeric helpers](../vendor/path-of-building-poe2/src/Modules/Common.lua) and
[ModScalability](../vendor/path-of-building-poe2/src/Data/ModScalability.lua) identify numeric
stages and per-component scaling. They inform offline conversion and independent oracle
tests; none becomes a native dependency or a general parser/interpreter specification.

## Candidate owned seams

The following API choices are proposals, not delivered behavior:

- Reuse `ItemRecord.parameters` for declared catalyst selection/presence and amount;
  `ItemRecord.quality` continues to represent ordinary quality. Known absence needs an
  explicit complete input/normalization proof; it is not inferred from any missing header.
- Reuse declared modifier parameters for a bounded set of known property predicates,
  category and affix-side selections. Preserve complete/partial membership. Do not create
  one fake modifier per label. If real callers need arbitrary property sets, introduce a
  small typed qualifier collection rather than string bags or actor capabilities.
- Add a separately named owning-item parameter read or explicit projection. Ordinary
  `Parameter` reads must continue requiring the exact program owner. The new seam must
  validate the actual ItemTemplate declaration reached through
  `ProviderRoot::ItemModifier { equipment_use, modifier }`, and the modifier must belong
  to that use's backing item. Do not copy catalyst values into each modifier during import:
  such copies would become stale after an item edit.
- Bind a bounded sequence of applicable magnitude transforms to each modifier occurrence.
  Reuse native arithmetic for Add/Multiply and Scale/Truncate. A generic commutative
  contribution reducer cannot replace an ordered transform sequence.

One backing item used twice keeps two equipment/modifier provider paths. Editing that
item's catalyst or roll affects both uses without changing their identities. A replaced
physical rune/provider follows normal occurrence replacement rules; rune text never creates
a Gem instance.

## Quality and ordered numeric semantics

Ordinary quality and catalyst quality are independent inputs. Ordinary weapon physical or
armour/evasion/energy-shield formulas must not scale cold resistance just because the item
has Quality20. Effective quality is also distinct from the authored amount.

Catalyst applicability matches **any** qualifying property; affix-side flags can participate.
Unscalable components retain scalar1. A recognized source catalyst with an omitted amount
uses20 only where the reviewed parser policy establishes that default; explicit0 remains0.
The source UI's separate Breach Ring authoring default50 is not an intrinsic runtime value
or permission to dispatch on an item name.

Magnitude selectors generally combine a category with **all** required labels. The reviewed
physical/chaos exception requires the damage label plus **any** of physical/chaos. Preserve
explicit aliases such as `defence` to `defences`; do not lowercase or merge unrelated labels
implicitly. Exclude inactive variants, unscalable components and skill-grant numbers as the
converted rule specifies. Rune members are not automatically in the ordinary magnitude
receiver set.

Start with the applicable catalyst scalar, then apply ordered signed additive percentages
or doubling operations. `(1 + 0.2) * 2` differs from `1 * 2 + 0.2`. Scale only the declared
numeric components, not condition thresholds or every number present in a line.

Preserve the stages: resolve the range at its declared internal precision; apply any
corrupted-base scalar with its distinct rounding; apply magnitude with truncation toward
zero; then convert precision for the semantic value. Existing Import
`InterpolateOffset`/`SymmetricHalfOffset` handles the reviewed source range stage. Moving a
stage into native rules requires its exact operation contract and tests; mathematical
nearest rounding is not a substitute for the source's literal half-offset operation.
Every stage and collection expansion remains bounded, with nonfinite results classified.

## Source encoding and compatibility limits

Some source formats carry nominal/base values; others carry values with scaling already
applied. Normalize that distinction through an explicit import encoding decision and
provenance. Never apply a scale twice or recover a nominal roll by dividing an already
rounded value. A source `advancedCopy` or editor-history flag is not an owned domain field.

The inspected source magnitude loop reparses only when its running scalar differs from1.
A sequence returning to1 may retain an earlier parsed value. This is an **oracle
investigation**, not an adopted domain rule: first establish the observable contrast, then
record the compatibility decision. Do not add mutable cache history to reproduce an
inferred quirk. The current ring/rune cases do not require that sequence.

## Concrete acceptance cases

Original05 Item26 is one Sapphire Ring with eight receiving rows, including two selected
uses. Its cold member retains the five labels `cold_resistance`, `elemental_resistance`,
`elemental`, `cold`, `resistance`, range fraction0.5 and nominal range20–30. Its second member
is `+10 to maximum Life`. No Catalyst/CatalystQuality, ordinary Quality or rune header is
present, but relevant header/producer completeness must establish absence. The original
stays Pending until the labels and relevant inputs are modeled. Existing tag-removed
in-memory copies remain explicitly diagnostic; they do not validate the original.

Test the original ring without stripping labels, unknown added labels/headers, exact
modifier membership, and all eight receiving rows. Contrast known absence, unresolved
catalyst, reviewed missing amount20 and explicit0. With explicitly established Tul20
applicability, nominal25 becomes30; ordinary Quality20 alone must leave cold unchanged.

Original02 supplies separate rune contrasts: helmet cold18 from GreaterGlacialRune plus
explicit cold27; two GreaterIronRunes represented by a single serialized36 defensive
member; and the Grand Spear's physical rune member and conditional Bonded member. Preserve
physical/source ownership and apply serialized or derived effects exactly once. Bonded
activation cannot be inferred from display text. Keep explicit attack speed49 distinct from
its26–28 affix metadata and keep the inactive primary weapon separate from the selected
swap weapon.

Also test catalyst ANY versus magnitude ALL/ANY, category/side/unscalable exclusions,
transform-order changes, signed truncation, unchanged grant levels, base versus already
scaled encodings, precision/aggregate limits and the source return-to-unity contrast.
Whole-item completeness, final resistance receivers and original-build parity remain
separate gates; this slice must not silently close them.
