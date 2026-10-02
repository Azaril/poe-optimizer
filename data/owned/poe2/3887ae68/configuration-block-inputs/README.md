# Saved enemy block input and Config BASE contribution

This family captures a finite source input and immediately feeds the native enemy
block consumer. It does not complete configuration roles, external assumptions,
whole enemy block membership, or any original build.

Pinned ConfigOptions.lua defines enemyBlockChance as countAllowZero with no input
or placeholder default. ConfigTab.Load stores numeric Input and Placeholder values
separately. BuildModList uses the Input whenever present, including zero, otherwise
the saved numeric Placeholder, including zero; neither produces no Config BASE
record. The callback emits BlockChance BASE on the enemy with source Config.

The opt-in PobFreshNumericConfigFallbacksV2 policy retains the existing six raw
rating/resistance rows exactly. Its separate placeholder_fallback_inputs list uses
an exact two-tier numeric recipe (Input then Placeholder), rejects duplicates and
malformed/ambiguous source values, and links the selected value to its exact source
row. The existing V1 contract and serialization remain unchanged. This lane is not
count-style zero fallback and does not import distance defaults.

| Meaning | ID |
| --- | --- |
| Selected saved numeric value is present | 3216 |
| Selected raw value, PercentagePoints | 3217 |
| Config BlockChance BASE contribution, Enemy context | 3218 |

The injected configured-enemy-block-base program lazily selects the raw value when
presence is true and zero otherwise. A demanded missing raw input or missing
presence remains unresolved. Raw admission is bounded to +/-1,000,000; this is not
a game clamp. Negative, fractional and above-100 values remain raw. The downstream
action consumer owns block aggregation, clamp/reduction and CannotBlockAttacks.
No non-Config contributions are inferred absent by this producer.

All five originals omit enemyBlockChance Input and Placeholder, so each gains one
Known false presence fact and no manufactured raw value. Existing issue counts,
source links, allocator state, saved selections and all 110 queries must remain
unchanged. Encounter31d1 keeps its Partial external-input and rule-program closure.
The extension adds two external definitions and one stat, plus one native program.

The complete fresh PoB witness passes 31 cases in both JIT modes, with identical
evidence recorded in authoring.json. Seven exact source-XML controls also exercise
the real importer during joint publication with the native minion hit-chance
consumer. dependencies.json preserves
all prior encounter input descriptors and their units for finite native fixtures.
