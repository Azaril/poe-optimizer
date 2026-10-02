# Incoming enemy damage inputs

This owned data family defines a native stage for the default Pinnacle encounter:
configured damage plus the enemy BASE Min/Max mean, its ordered total, and three
elemental penetration inputs. The stage precedes conversion, gain, damage
multipliers, critical effect and player mitigation. It does not calculate EHP.

`extension.json` supplies the schemas, level-indexed damage table and one typed
rule program. `policy.json` adds exact raw numeric overrides and a finite damage
category option to the existing import profile. `native-inputs.json` describes
the test/publication joins. Runtime Rust has no PoB names, game constants or build
identities for this family. No database or Lua execution is required by the native
program.

Raw zero, negative and fractional inputs retain their meaning within the declared
bounded domain. PoB's Pinnacle callback overwrites saved damage and penetration
placeholders; these are not authoritative fallbacks. The program computes defaults
from the injected table and constants, preserving sequential multiplication and
both rounding operations. Overriding Physical does not alter the Chaos default.
Damage-over-time bypasses this hit stage and returns zero. Other admitted categories
share this stage; their later avoidance/mitigation calculations are separate.

Min/Max channels require complete incoming contributor membership before reducing
them. An empty contribution set is not inferred from a zero reference total.
The real Encounter remains Partial. Only the explicit finite component fixture
closes its test-world membership; it does not admit any of the five full builds.

Source authentication and publication are recorded in `authoring.json`. A pending
validation status is not permission to publish. The optional PoB source witness
checks the full original Build lifecycle without replacing game methods. Default
native tests execute this exact owned program; source comparison authenticates
measured vectors separately.

The checked source witness passes 33 cases in each JIT mode. Default native tests
replay 62 measured MAIN/CALCS vectors. Publication preserves all five original
selections and all 110 queries; complete native original builds remain 0/5.
See the [active implementation checkpoint](../../../../../docs/implementation.md)
for the checked package, validation commands and next blockers.
