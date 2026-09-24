# Ordinary rarity and Chaos Resistance inputs

Two distinct owned families extend the ordinary item input path. Item rarity represents an
increase/reduction, while Chaos Resistance represents a signed flat amount. Source witnesses
record LootRarity/INC and ChaosResist/BASE; the native evaluator consumes typed owned rules,
not those source names. Neither family emits an Actor contribution or closes item coverage.

## Numeric and occurrence contract

Rarity owner `29bc` has 24 required inputs: a nonnegative magnitude, explicit negative
qualifier `29d4`, twenty property predicates, line scalability state and corrupted-base
factor. Chaos owner `29d5` retains 23 inputs with a signed amount. Both use percent unit2
and existing Modifier output253e. Each keeps its own occurrence; shared stat definitions
never merge equal values from different source lines.

The generic value compiler uses precision1/display0. Rarity applies its qualifier after
internal rounding, corrupted-base rounding, magnitude truncation and final display rounding.
For magnitude10, corruption2 and magnitude1.5, reduced rarity yields -30. Direct signed
Chaos -10 under those factors yields -28 because the source corruption rounding differs.
Signed zero retains its qualifier direction during raw projection before quantity zero is
canonicalized. Missing qualifier or scalar inputs remain unresolved.

The source grammar accepts raw decimal/signed rarity captures using magnitude/direction
projections; only unsigned integer increased/reduced text in 0..1,000,000 receives the new
source-member proof. Chaos admits ordinary plus-prefixed integers under the same bounds;
negative/decimal raw Chaos remains unproved. All three guarded grammars require no source
tags and no generated base prefix. No property is inferred from English text. Actual chaos
properties may select Chayula's catalyst; there is no inferred rarity catalyst category.

Generated Golden Charm rarity15 and Amethyst Charm Chaos18 can suppress matching supplied
text. Source tests observe those exceptions; this package retains the stricter generated-
prefix exclusion. Tagged, crafted, rune, conditional, range and predecessor-ambiguous lines
receive no broader authority. Saved rune text and rune reconstruction are separate source
phases: two parser calls can still leave one physical modifier occurrence.

## Artifacts and validation boundary

The 90,761-byte extension allocates 49 entries and eight programs. The 199,332-byte finite
membership patch adds two explicit structural members to all 1,756 known base templates
(3,512 insertions), replacing a 9,516,263-byte fully expanded authoring file. It preserves
all prior definitions, programs, members and Partial closures. Membership does not establish
affix legality. Published runtime schemas remain materialized; compact authoring is not a
runtime memory optimization.

Publish from the exact attribute-input predecessor with `extend-owned-recipe`, supplying
`extension.json`, `membership-patch.json`, paired `items.json`/`item-source.json`, and a new
output directory. `bindings.json` retains exact identities and source witnesses. The shared
[Rust authoring contract](../../../../../docs/owned-recipe-membership.md) rejects stale
endpoints and never overwrites publication directories.

The optional Rust source suite covers all 1,756 constructed bases, both rarity qualifiers,
Chaos amounts, numeric order, duplicate generation, catalysts and all 27 original source
rows. Reusable native test helpers retain attribute Count/compound checks while adding
rarity sign/zero/absence contrasts, exact source occurrences, publication safety and all-five
query/coverage preservation. Complete native original-build parity remains a separate gate.