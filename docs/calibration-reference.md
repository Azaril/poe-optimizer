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
`Launch.lua` directly. It imports fixture XML through `main:SetMode`, checks
initialization, clears PoB's global cache, and calls
`character.calcsTab.calcs.buildOutput(character, "MAIN")` directly. It selects
19 finite numeric values from the returned player environment. Imported cached
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

Both fixtures contain a level 60 Sorceress with no ascendancy, equipment, support
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

Additional attack, item/support interaction, minion, and survival-mechanic cases
remain necessary before broad evaluator confidence or optimizer claims.