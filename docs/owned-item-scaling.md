# Item modifier properties and scaling

**Status: modifier-property conversion implemented; catalyst/header and effective scaling remain open.**
The owned source adapter converts the untouched original05 ring's nominal cold roll and
five Boolean modifier properties. New nominal definitions retain Partial numerical rules;
they do not reuse the prior fixed-value contribution programs. Two
[Engine regressions](../crates/poe-optimizer-engine/tests/owned_item_properties.rs) validate
the existing Stat dependency path across templates and repeated item uses. These seams
are ready for real scaling data, but the whole ring remains Pending, and full
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

The common-property dependency path is validated with directly authored data. Source
property conversion is implemented through injected item-line/source policies v2: bounded
source tokens map to local Import keys, then to schema-bound Boolean modifier parameters.
Unknown or unconsumed labels block the line; source strings do not enter native rules.
The remaining header, catalyst and scaling work follows these seams:

- Reuse `ItemRecord.parameters` for declared catalyst selection/presence and amount;
  `ItemRecord.quality` continues to represent ordinary quality. Known absence needs an
  explicit complete input/normalization proof; it is not inferred from any missing header.
- Reuse declared modifier parameters for a bounded set of known property predicates,
  category and affix-side selections. Preserve complete/partial membership. Do not create
  one fake modifier per label. If real callers need arbitrary property sets, introduce a
  small typed qualifier collection rather than string bags or actor capabilities.
- Establish a shared semantic equipment-property contract before adding a cross-owner
  parameter read. A Modifier definition can occur on different ItemTemplates; a read of
  one template's declared slot cannot represent every occurrence, and equal slot names
  never authorize a match. Reuse the validated path: an ItemTemplate-owned program
  in EquipmentUse context reads its exact authored parameters and derives named effective
  equipment properties; a shared Modifier program in that same equipment context consumes
  them through existing Stat dependencies. The concrete ItemSlotUseId determines the
  supplying item. Ordinary `Parameter` reads keep their exact-owner requirement.
- Use that existing dependency path only for explicit semantic properties, such as catalyst
  applicability inputs and effective amount, with declared types, units and coverage. It is
  not a bag of source fields or a substitute for arbitrary parameter transport. An
  Actor-context modifier cannot implicitly read its equipment through Current; any needed
  relationship must be designed explicitly. Computed Stat types also do not supply the
  allowed-option/range constraints of authored ValueSchema inputs. If these limits prevent
  a real consumer, compare a small typed item-to-modifier projection with an explicit
  common-input binding before adding API. Do not copy mutable item values into imported
  modifier records: those copies would become stale after an item edit.
- Keep occurrence-specific roll/scaling intermediates local to each modifier program.
  Repeated occurrences may contribute to the same target, but must not each derive the
  same final equipment-property Stat. That is a competing-producer error, not an
  aggregation rule. Player contributions use an explicit Player target; this equipment
  contract does not establish item-to-minion or actor-relative recipient transport.
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

Start with the applicable catalyst scalar, preserving the source grouping
`(100 + quality) / 100`, then apply ordered signed additive percentages or doubling
operations. The optional pinned oracle confirms a meaningful floating-point contrast:
quality0.7 with base1000 truncates to1007 with that grouping, but to1006 with
`1 + quality / 100`. Existing native Add and percentage-to-factor operations can express
the required grouping; no new interpreter or operation is required. `(1 + 0.2) * 2` differs from `1 * 2 + 0.2`. Scale only the declared
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

The optional pinned oracle now confirms a source-history contrast: for a fixed25 line,
ordered +20% then -20% can retain30 when the scalar returns to1; the reverse order can
retain20. The ranged form recomputes the final25 during the source build stage. This is
reference evidence, not an adopted domain rule. Keep the fixed/baked encoding compatibility
decision open and do not add mutable cache history to native evaluation. The original05
ring uses the ranged path and does not require that quirk. The source-only test lives in
[owned_catalyst_oracle.rs](../crates/poe-optimizer-pob/tests/owned_catalyst_oracle.rs); its31
contrasts do not establish native scaling or whole-build parity.

## Concrete acceptance cases

Original05 Item26 is one Sapphire Ring with eight receiving rows, including two selected
uses. Its cold member retains the five labels `cold_resistance`, `elemental_resistance`,
`elemental`, `cold`, `resistance`, range fraction0.5 and nominal range20–30. Its second member
is `+10 to maximum Life`. No Catalyst/CatalystQuality, ordinary Quality or rune header is
present, but relevant header/producer completeness must establish absence. The production
converter now retains nominal cold25 and all five true properties from the untouched
original, retains its Life input and all eight uses, and keeps the new nominal family
Partial. The source-layout proof does not establish whole-item completeness. The former
tag-stripped test helper has been replaced by the actual original test; conversion of the
original is no longer represented by a diagnostic copy.

The component regressions validate two distinct ItemTemplates and their exact parameter
declarations feeding the same Modifier definition, repeated uses of one backing item, an
unrelated item's inputs, missing/zero/absent amounts and inactive equipment. Retain these
contrasts when binding real data, including the existing competing-producer checks.
Changing a backing item's parameters must update its uses without changing provider
identities or borrowing from another template. Shared effects must retain their exact
modifier occurrences and actor target. This is a structural test, not catalyst parity.

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
