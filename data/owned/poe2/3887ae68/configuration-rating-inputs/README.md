# Configured Enemy Armour and Evasion BASE inputs

This family uses the canonical scenario enemy level and injected raw monster
tables to calculate the two Pinnacle configuration defaults. It adds no duplicate
enemy-level input. Rule operations V14 introduces the typed `EnemyLevel` read;
existing maximum, table lookup, percent-to-factor, scale, round and lazy select
operations express the calculation:

`round(monsterTable[max(enemyLevel, 82)] * (pinnacleMean / 100))`

The minimum level, both complete level 1–85 tables, means, units and programs are
data in `extension.json`. The means are 150 and 124.9090909090909 percentage points,
converted to dimensionless factors before scaling. Rounding is to one Rating,
nearest with ties toward positive infinity, matching the pinned source order.
No source-calculated Armour/Evasion default is imported as an input constant.

| Family | Presence | Raw Rating | Contribution |
| --- | --- | --- | --- |
| Armour | 31de | 31df | 31e0 |
| Evasion | 31e1 | 31e2 | 31e3 |

`policy.json` preserves the predecessor's four resistance overrides and appends
the two rating overrides using the existing generic numeric Config adapter.
Raw quantities are bounded to ±1,000,000 as adapter admission scope, not a game
clamp. Explicit zero, negative and fractional values bypass the default calculation.
Missing presence or a demanded missing raw quantity stays unresolved. All units
already exist in the predecessor: Rating `29ee`, PercentagePoints `0002` and
DimensionlessFactor `0001`.

Encounter `31d1` gains four declared external-input members, two programs and two
ordinary rule tables. Its previous Partial membership and rule-coverage gaps
remain intact. Callbacks at lines 2183–2187 in pinned ConfigOptions.lua emit **Config-owned**
BASE records. This differs from the preceding resistance family's EnemyConfig
source and must be joined correctly in reference comparisons.

The complete-source witness is the separate `owned_enemy_ratings_source` target.
Authoring remains pending until that witness passes. Component validation checks
both real source delivery and the injected native calculations. These producers
do not calculate hit chance, physical reduction, final metrics or a whole build.
