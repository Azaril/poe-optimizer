# Calibration fixtures

These tiny self-cast Spark builds have no equipment, support gems, supporting
skills, or explicitly allocated passives. `spark-mapping.xml` selects a normal
level 60 enemy with 0% elemental resistance; `spark-bossing.xml` selects a level
82 Pinnacle enemy with 50%. Incoming hits and other supplied assumptions are
explicit in each XML.

The neighboring `*.reference.json` files were produced by the separate Windows
C/Lua reference host using PoB's bundled DLLs, not by the Rust evaluator.
Each Spark reference contains 19 raw numeric expected outputs, comparison tolerances, and full
source/runtime/generator provenance. Both input and JSON line endings are fixed
to LF for reproducible fixture hashes.

See [calibration-reference.md](../../../docs/calibration-reference.md) for the
method, assumptions, limitations, and regeneration instructions. These cases
validate host/extraction parity against the shared PoB calculations; they do
not independently certify game mechanics or represent realistic endgame builds.
The `mace-*.xml` files add a level 60 Warrior with a single one-handed Mace Strike
and one normal, quality-zero weapon. The four cases cross Wooden Club / Smithing
Hammer with no support / Brutality I. All use the same normal level 60 enemy,
zero enemy armour/resistances, and incoming physical hit of 1,000. Both weapon
requirements are satisfied by the fixed base character attributes. Skill level
is held at one for calculation calibration; complete in-game legality is not
asserted. The implicit Warrior start has known missing upstream edges, and no
explicit passive paths are taken.

The preferred weapon reverses when Brutality removes fire damage. Each attack
reference contains 25 top-level metrics, nine main-hand values, and the weapon
requirements, damage ranges, and applied support IDs observed in the independent
host. These goldens use `scripts/reference-attack-calibrate.ps1` and the separate
attack extractor; the original Spark scripts and goldens remain unchanged.
Production currently exposes typed hit DPS for these attacks but explicitly
reports selected average hit unavailable until per-hand semantics are defined.
