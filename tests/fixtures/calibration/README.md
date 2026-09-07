# Calibration fixtures

These tiny self-cast Spark builds have no equipment, support gems, supporting
skills, or explicitly allocated passives. `spark-mapping.xml` selects a normal
level 60 enemy with 0% elemental resistance; `spark-bossing.xml` selects a level
82 Pinnacle enemy with 50%. Incoming hits and other supplied assumptions are
explicit in each XML.

The neighboring `*.reference.json` files were produced by the separate Windows
C/Lua reference host using PoB's bundled DLLs, not by the Rust evaluator.
Each contains 19 raw numeric expected outputs, comparison tolerances, and full
source/runtime/generator provenance. Both input and JSON line endings are fixed
to LF for reproducible fixture hashes.

See [calibration-reference.md](../../../docs/calibration-reference.md) for the
method, assumptions, limitations, and regeneration instructions. These cases
validate host/extraction parity against the shared PoB calculations; they do
not independently certify game mechanics or represent realistic endgame builds.