# Independent PoB host calibration

This reference checks whether the Rust evaluator loads a small build and extracts
the same fresh numbers as a separate host. Both hosts call the same pinned PoB
calculation engine. Agreement does **not** independently certify game mechanics,
build legality, broad mechanic coverage, or the meaning of every exposed metric.

## Reference path

The [C driver](../scripts/reference-driver.c) loads the pinned PoB distribution's
`runtime/lua51.dll`. Lua then loads that distribution's matching
`runtime/lua-utf8.dll`. Neither the Rust executable nor its
`runtime.rs`, `host.lua`, or `snapshot.lua` participates in reference generation.

The [independent Lua harness](../scripts/reference-pob.lua) loads upstream
`_SimpleGraphic.def.lua`, supplies its own minimal headless callbacks, and boots
`Launch.lua` directly. It completes the original item-database loading callback before
importing fixture XML through `main:SetMode`, checks initialization, clears PoB's global
cache, and calls
`character.calcsTab.calcs.buildOutput(character, "MAIN")` directly. It selects
19 finite numeric values for the original Spark references. The separate attack
extractor below additionally records weapon and per-hand details. Imported cached
`PlayerStat` values and the production adapter's saved output tables are unused.

Reference runtime measured on 2026-09-07:

- PoB source: `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`.
- Bundled interpreter: `LuaJIT 2.1.1784580905`, Lua 5.1, x64.
- Driver compiler: MSVC `19.50.35723.0`, `/O2`, x64.
- Host: Windows `10.0.26200.0`.
- The bundled interpreter parses the pinned entry points, including augmented
  assignment. [reference-probe.lua](../scripts/reference-probe.lua) records the
  small standalone syntax probe.
- Production currently uses the separately vendored mlua LuaJIT build
  `2.1.1787165859`. The two paths share PoB calculations while using distinct
  host code, extraction code, and LuaJIT builds.

Each reference JSON records the exact fixture, source manifest, DLL, harness,
driver source/executable, and generator hashes. DLL binaries remain in the
existing upstream submodule. No new native modules are loaded by production.

## Small fixtures and assumptions

Both Spark fixtures contain a level 60 Sorceress with no ascendancy, equipment, support
gems, supporting skills, or explicitly allocated passives. They select level 1,
quality 0 Spark, one skill group, one count, and include that group in Full DPS.
These intentionally weak characters isolate import and evaluation behavior;
they are calibration cases rather than representative optimized endgame builds.

| Assumption | Mapping fixture | Bossing fixture |
| --- | --- | --- |
| Enemy preset | Normal (`None`) | `Pinnacle` |
| Enemy level | 60 | 82 |
| Enemy fire/cold/lightning resistance | 0% | 50% |
| Enemy chaos resistance | 0% | 0% |
| Incoming damage category | Melee | Melee |
| Incoming physical hit | 1,000 | 3,000 |
| Incoming elemental/chaos damage, penetration, overwhelm, crit chance | 0 | 0 |
| Incoming action time | 1,000 ms | 1,000 ms |
| Nearby enemies / nearby rare or unique enemies | 1 / 0 | 1 / 1 |

Enemy shocked/chilled/ignited and recent-hit/recent-crit assumptions are explicitly
false. Remaining options use the recorded PoB revision's defaults; the XML
contains all explicitly supplied inputs. PoB's remaining default character and
quest settings produce -50% player elemental resistance in these reference
runs; the fixture does not override them. The mapping case is a normal-monster
calculation proxy, not a clear-speed or multi-target simulation. Spark's
projectile geometry, repeated hits, and encounter uptime are not modeled by this
fixture.

| Selected output | Mapping | Bossing |
| --- | ---: | ---: |
| AverageHit | 5.995 | 2.9975 |
| TotalDPS / CombinedDPS / FullDPS | 8.5642857142857 | 4.2821428571429 |
| Speed | 1.4285714285714 | 1.4285714285714 |
| CritChance | 9 | 9 |
| Life / Mana | 809 / 315 | 809 / 315 |
| TotalEHP / PhysicalMaximumHitTaken | 809 / 809 | 809 / 809 |

The damage-halving response to the explicitly changed enemy resistance is a
useful scenario sanity check. EHP equality here is expected for a character with
no avoidance or mitigation; it is not evidence that boss and mapping
survivability are generally interchangeable.

Two independent fresh processes per fixture produced identical selected
reference metrics. Upstream emits existing `missing node` diagnostics during
tree initialization; the generator preserves stdout/stderr logs locally.
These fixtures spend no passive points, and those diagnostics are not suppressed
or presented as evidence of full tree-mechanic coverage.

## Regeneration and regression use

From a clean pinned submodule checkout on Windows x64, with PowerShell 7.2+ and
Visual Studio C++ Build Tools available:

```powershell
pwsh -File scripts/reference-calibrate.ps1
```

The script discovers MSVC with `vswhere`, compiles the separate driver, creates
fresh scratch directories, and runs each reference process with a 45-second
budget. Results, executable, scratch directories, and logs go beneath a new
`local/reference-calibration/run-*` directory. The current small C driver uses
narrow Windows paths; the generation script explicitly requires ASCII paths.
This reference limitation does not define the production path contract.

The 2026-09-09 readiness repair added explicit completion of the upstream unique/rare
loading task to both independent harnesses. A single initialization frame does not ensure
all prototypes are available for imported-item requirements. Fresh runs of all six
fixtures retained every non-provenance output field, including 138 measurements. The
committed goldens preserve their original generator/harness identities; newly generated
outputs record the repaired harness hashes. Validation and evidence are recorded in the
[readiness checkpoint](implementation.md#reference-item-database-readiness-checkpoint).

The script never runs the Rust evaluator or rewrites committed expected numbers.
Review generated metrics and provenance before copying a new reference JSON
into [tests/fixtures/calibration](../tests/fixtures/calibration).
Do not generate new expected values from the system under test.

Regression tests on both Windows and Linux can load the committed JSON, evaluate
the corresponding XML using production, require the recorded skill/build
identity, and compare every recorded metric using:

```text
abs(actual - expected) <= max(1e-8, abs(expected) * 1e-9)
```

The tolerance accommodates serialized decimal precision and small platform/runtime
floating-point differences; it must not conceal an unexplained mechanic change.
Compare fixture hashes against the reference provenance before using a golden.
Changing an upstream pin, fixture, driver, or extractor requires an explicit
reference refresh and an [implementation checkpoint](implementation.md).

Additional minion, dual-wield, support-family conflict, passive-tree, and survival-
mechanic cases remain necessary before broad evaluator confidence or optimizer
claims.
## Attack, weapon, and support interaction

Four additional fixtures form a controlled two-by-two comparison. A level 60
Warrior with no ascendancy or explicitly allocated passives uses level 1,
quality 0 **Mace Strike** (`Melee1HMacePlayer`). The only equipped item is a
normal, quality 0 one-handed mace in Weapon 1; the off hand is empty. Each case
uses either no support or level 1, quality 0 **Brutality I**. There is one active
skill group, explicitly included in Full DPS with count one. This intentionally
small calculation state holds the skill level fixed; it is not a certified
in-game progression or legal endgame build.

The attack cases use the Spark mapping fixture's normal level 60 enemy and
incoming physical hit of 1,000, with **enemy armour explicitly set to zero** and
all elemental/chaos resistances zero. This isolates damage types from enemy
armour's nonlinear response. Accuracy is left to PoB: the recorded accuracy is
396, hit chance 86%, effective critical chance 4.3%, and action rate 1.45 per
second. The same assumptions apply to all four cases. They do not measure clear
speed, boss uptime, additional strike targets, or real combat execution.

| Fixture | Weapon base damage | Brutality I | Raw selected TotalDPS | Reference main-hand AverageHit |
| --- | --- | --- | ---: | ---: |
| `mace-wooden` | Wooden Club: 6–10 physical | No | 10.404968 | 8.344 |
| `mace-smithing` | Smithing Hammer: 5–9 physical, 5–9 fire | No | 18.208694 | 14.602 |
| `mace-wooden-brutality` | Wooden Club: 6–10 physical | Yes | 13.6565205 | 10.9515 |
| `mace-smithing-brutality` | Smithing Hammer: 5–9 physical, 5–9 fire | Yes | 11.0552785 | 8.8655 |

The Smithing Hammer wins without Brutality; the Wooden Club wins with it.
Brutality improves the Wooden Club's DPS by 3.2515525 but reduces the Smithing
Hammer's DPS by 7.1534155. This is a concrete interaction that cannot be modeled
by summing independent item and support scores. The cross difference between
both changes is -10.404968 DPS. These values include PoB's rounding of physical
damage endpoints: multiplying the Wooden Club's 6–10 range by 1.25 yields 8–13
after rounding, while the Smithing Hammer's 5–9 becomes 6–11. A blanket 25%
multiplier applied to the final DPS would give the wrong answer.

The reference asserts the active skill's exact ID and reads **applied** support
effects from the main skill's `effectList`, where upstream has already checked
support compatibility. It records weapon base/type/quality, base requirements,
skill level, damage endpoints, and the main-hand numeric output. The Warrior's
15 strength satisfies the Smithing Hammer's base requirement of 11; the Wooden
Club has no base attribute requirement. These focused checks do not establish
full equipment, resource, socket, support-count, or passive legality.

The Warrior's implicit start node is 47175. The pinned tree has three missing
edges originating at that allocated start; the fixtures take no explicit tree
paths. The cases retain those diagnostics and **do not validate Warrior tree
connectivity**. See [skill coverage](skill-coverage.md) for the wider upstream
graph issue.

### Separate, reproducible attack reference path

The new [attack generator](../scripts/reference-attack-calibrate.ps1) compiles
the existing independent C driver and uses a separate
[attack Lua extractor](../scripts/reference-attack-pob.lua). It has method ID
`bundled-dll-independent-host-direct-main-attack-v1`, records 25 top-level
numeric metrics plus nine main-hand values, and checks the same PoB pin/runtime
as the Spark reference. It never invokes the Rust evaluator. Keeping separate
scripts preserves the **original Spark generator, extractor, XML, and goldens
byte-for-byte**, including their original provenance hashes.

```powershell
pwsh -File scripts/reference-attack-calibrate.ps1
```

Each fresh run creates `local/reference-calibration/attack-*`, with a separate
scratch directory/process for every fixture. The output JSON records fixture,
DLL, C driver, extractor, generator, and source manifest hashes. Two fresh
processes per fixture matched all 25 numeric metrics and the complete attack
detail block exactly before the references were copied into the repository.
The committed outputs were produced on 2026-09-07 with the Windows x64 bundled
LuaJIT and MSVC versions listed above. Regeneration follows the same review and
tolerance policy as Spark; expected values must never come from production.

The [attack regression test](../tests/attack_calibration.rs) checks all 25
recorded top-level outputs against production, exact fixture/source identity,
resolved skill/support entries, physical weapon endpoints, the explicit enemy
configuration, and both directions of the weapon/support ranking change. It
also checks the typed selected hit DPS and its unit. The independent main-hand
values remain reference evidence; production does not yet expose a typed
per-hand output contract.

This exposed a useful mapping gap: PoB stores attack `AverageHit` in
`output.MainHand`/`output.OffHand`, whereas the current selected-average-hit
binding reads a top-level scalar. Therefore **`player.selected_average_hit`
remains explicitly unavailable for these attacks**, even though hit DPS is
available. The test preserves this honest availability contract. A future
mapping must define the single-hand, alternating-hand, and simultaneous-hand
semantics before exposing a combined value; it must not choose a hand silently.
