# Ordinary weapon modifier inputs

This authored extension adds nine modifier families and three Modifier-scoped nominal
output channels. It admits 23 explicit line grammars for physical increase, attack
speed, critical chance, flat critical chance and five flat damage pairs. Definitions,
roll schemas, applicable base templates, source properties and arithmetic are injected
data; there is no runtime dispatch on a fixture or item display name.

Apply to the explicit-header successor with `extend-owned-recipe`, supplying this
folder's `extension.json`, `items.json` and `item-source.json` together. These policies
bind the exact successor schema, so unrelated package versions cannot be substituted.
The extension appends 263 registry entries and nine programs, and enriches modifier
membership on exactly the 337 existing weapon-profile templates. An Emerald or amulet
with similar wording is not admitted as a weapon-local modifier.

The captured amount is a nominal textual magnitude. Increased/reduced families carry
a required Boolean `reduced` roll; their native recipes apply its sign **after** ranged
interpolation and rounding. Fixed signed values remain signed. Applying a negative
capture scale to both range endpoints would reverse their ordering and is incorrect.
Flat critical chance retains decimal precision; ordinary flat pairs retain their
separate minimum and maximum values. Mixed fixed/ranged endpoints are not admitted yet.

The v4 source policy preserves explicit fractured/desecrated flags through reviewed
bindings, alongside typed property facts. The ordinary grammar declares `unscalable`
false; unknown source flags do not receive that default. No arbitrary source flag is
executed as a rule. Source-line attribution, variant selection, combined-line and rune
lifecycle gates remain in force before owned modifier admission.

These programs derive **nominal** Modifier stats only. They make no effective stat
contribution and retain the three gaps `source-roll-encoding-unproved`,
`numeric-component-scaling-unconverted` and `effective-magnitude-unconverted`.
Catalyst/corruption scaling, ordered add/scale operations, crafted state and rune
additional-effect rounding are separate work. Standard weapon quality belongs to the
later weapon channel assembly; it must not change a nominal 167% roll into 200% here.

Rust tests cover every grammar, ranged endpoints/midpoints, sign contrasts, missing
facts, native scratch reuse, exact source flags, non-weapon rejection, immutable prior
publication, no-overwrite and all five original imports. Their 110 query rows remain
unchanged and complete native original-build coverage remains 0/5.
