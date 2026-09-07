# Path of Building PoE2 integration notes

Source investigation recorded on 2026-09-07. No Lua runtime or calculation was executed during this investigation. All upstream references below are pinned to submodule commit `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`. These dated observations inform the [target design](design.md); runtime findings, current validation status, and follow-up work belong in the [living implementation document](implementation.md).

The source already provides a useful headless entry point and a comparison calculator. Preserve upstream calculations and establish an independently reloaded build as the correctness baseline before enabling incremental evaluation. The subsequent user-confirmed [hosting decision](design.md#evaluator-boundary) prefers a Rust worker embedding LuaJIT through `mlua`, with a small Lua compatibility shim; direct LuaJIT execution is a diagnostic fallback.

## Runtime checkpoint: embedded host

The subsequent mlua spike successfully built and ran the pinned entry points on Windows
with Rust 1.93.0, mlua 0.12.1 and luajit-src 210.7.3+1ee778a (reported runtime
`LuaJIT 2.1.1787165859`). The leading hash lines and the `count += 1` statement parse
without source overlays in this runtime. The earlier syntax concern is therefore not a
reproduced blocker for the selected host; no PoB source patch was applied.

The native bridge compiles luautf8 0.1.6 against matching public LuaJIT headers and
preloads its C entry point into the same VM. It loads neither bundled Windows DLL.
See its [provenance](../crates/poe-optimizer-lua-utf8/README.md) and native tests.

The supplied minion build produces fresh MAIN player/minion snapshots and XML export.
Three source entries remain unresolved, and no groups participate in Full DPS. The raw
selected-minion TotalDPS matches the source cache, but that consistency is not independent
parity evidence. The adapter reports unvalidated coverage and explicit non-finite fields.
Source hashing, import-completion guards, fresh calculation revisions and supervisor-owned
scratch storage address the startup and lifetime issues found during implementation review.
PoB can also load a fresh default character when Build/Tree data is absent, so evaluator
preflight requires explicit supported identity and rejects duplicate core sections.
Container-only import remains lossless and separate from these evaluator restrictions.
Current tests and remaining acceptance work are in the [implementation record](implementation.md).

## Verified source facts

### Bootstrap and runtime dependencies

- [HeadlessWrapper.lua:1-78](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/HeadlessWrapper.lua#L1-L78) loads `_SimpleGraphic.def.lua`, provides callback dispatch, suppresses `require("lcurl.safe")`, loads `Launch.lua`, then executes initialization and one frame. It exposes global `build`, `newBuild()`, `loadBuildFromXML(xmlText, name)`, and `loadBuildFromJSON(characterJSON)`.
- Upstream [`.busted`](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/.busted#L1-L13) runs with `src` as the working directory and includes `../runtime/lua/?.lua;../runtime/lua/?/init.lua` in the Lua module path. Relative file loading is part of the current integration contract.
- [Launch.lua:19](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Launch.lua#L19) unconditionally invokes `jit.opt.start`. [Common.lua:19-30](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Modules/Common.lua#L19-L30) expects global `bit`, bundled `xml`, `base64`, and `sha1`, and the native `lua-utf8` module. Do not interpret the wrapper's old “standard lua interpreter” comment as a tested compatibility guarantee.
- [Dockerfile](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/Dockerfile#L1-L40) builds Lua 5.1.5, a pinned LuaJIT revision, and `luautf8`; [test.yml:30](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/.github/workflows/test.yml#L30) uses `busted --lua=luajit`. The inspected checkout includes Windows `runtime/lua51.dll` and `runtime/lua-utf8.dll`, but an executable Lua interpreter was not found on the investigation shell's PATH. This investigation did not establish ABI and architecture compatibility with an embedded Rust runtime.

### Embedding bootstrap checks

Follow-up source/binary inspection on 2026-09-07 for the mlua hosting decision found:

- The bundled [lua-utf8.dll](../vendor/path-of-building-poe2/runtime/lua-utf8.dll) and
  [lua51.dll](../vendor/path-of-building-poe2/runtime/lua51.dll) have x64 PE headers, and
  the former imports the latter. This establishes a linkage dependency, not compatibility
  with a vendored/static embedded runtime. Prove a single compatible runtime or rebuild/
  register the native module against the selected host before loading real builds.
- [Main.lua:63](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Modules/Main.lua#L63)
  reads `arg[1]`; provide the interpreter-style `arg` table during embedded bootstrap.
- `HeadlessWrapper.lua` and `Launch.lua` begin with `#@` lines. Verify their treatment when
  loading through mlua, or preserve file-loader behavior deliberately. The existing
  `count += 1` concern remains separate; changing the Rust binding does not change the
  selected LuaJIT language syntax.
- The wrapper loads `_SimpleGraphic.def.lua` before `Launch.lua`, overwriting matching
  host functions. Install real callbacks after those definitions and before application
  initialization; test diagnostics, paths, clock and prompt handling in that sequence.

These are inspection findings only. Runtime/module loading and calculation checks remain
in the [implementation checklist](implementation.md#m1-checklist-native-calculation-and-optional-reference-parity).

### Load, mutate, calculate, and export

| Operation | Existing seam | Implication for the adapter |
| --- | --- | --- |
| Load a build | `loadBuildFromXML` in the headless wrapper | Prefer raw PoB XML as the initial input. JSON character import explicitly leaves the main skill and configuration unset. |
| Configure a build | [`ConfigTab:BuildModList()`](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Classes/ConfigTab.lua#L1169) | Set controlled inputs, rebuild their mod list, and recalculate. Existing [defence tests](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/spec/System/TestDefence_spec.lua#L37-L44) demonstrate this pattern. |
| Mutate passives | [`PassiveSpec:AllocNode`, `DeallocNode`, `CountAllocNodes`](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Classes/PassiveSpec.lua#L954-L1055) | Allocation can add a complete path; removal can delete dependent nodes. Counts distinguish ordinary, ascendancy, secondary ascendancy, and weapon-set points. Capture the resulting allocation, not merely the requested node. |
| Parse and add items | [`CreateDisplayItemFromRaw`](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Classes/ItemsTab.lua#L1970) and [`AddDisplayItem(noAutoEquip)`](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Classes/ItemsTab.lua#L1781) | Reuse item parsing; make intended slot and item-set selection explicit. Adding an item and equipping it are separate concerns. |
| Recalculate like the application | [`Build:OnFrame` dirty-build branch](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Modules/Build.lua#L1387-L1401) | When `build.buildFlag` is true, it clears the global calculation cache, increments output revision, and calls `BuildOutput()`. Merely mutating a Lua table does not establish that fresh outputs exist. |
| Read displayed metrics | [`CalcsTab:BuildOutput`](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Classes/CalcsTab.lua#L486-L517) | `build.calcsTab.mainOutput` is `mainEnv.player.output`, calculated in `MAIN` mode. `calcsOutput` comes from a separate `CALCS` environment. Pin the chosen mode in results. |
| Recalculate through a lower seam | [`calcs.buildOutput(build, mode)`](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Modules/Calcs.lua#L469-L482) | It initializes the environment, performs calculations, and adds full-DPS roll-up fields. Benchmark this only after matching the ordinary frame path. |
| Export a candidate | [`build:SaveDB(fileName)`](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Modules/Build.lua#L2746-L2769) | Returns composed `PathOfBuilding2` XML without itself writing it. Use this for reviewable results and round-trip validation. |

### Existing optimization assistance

[`calcs.getMiscCalculator(build)`](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Modules/Calcs.lua#L73-L144) returns a baseline output and a function that evaluates overrides. Documented overrides include `spec`, `addNodes`, `removeNodes`, `repSlotName`, `repItem`, flask/charm toggles, and conditions. Ordinary passive overrides use **node objects as table keys**, not just numeric IDs; anointed nodes are a documented exception. Rust should send stable IDs and let the Lua adapter resolve objects.

[`CalcsTab:PowerBuilder()`](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Classes/CalcsTab.lua#L540-L698) already evaluates single-node changes, full allocation paths, and dependent-node removals. This is a candidate-ranking aid, not a global constrained optimizer. Its [combined defence heuristic](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Classes/CalcsTab.lua#L707-L716) combines several pool, mitigation, and recovery deltas rather than directly measuring `TotalEHP`; do not silently reuse it as the user's EHP objective.

The miscellaneous calculator also has optional accelerated environment reuse and full-DPS caches. `fullDPSOnly` returns a small DPS-only table; another path can pass `skipEHP`. These options cannot certify an EHP constraint. Full re-evaluation of retained candidates remains necessary until equivalence is demonstrated for each supported mutation family.

### State and error behavior

`newBuild()` explicitly wipes global calculation caches; the ordinary dirty-build frame does too. The clearing function [`wipeGlobalCache()`](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Modules/Common.lua#L950-L954) clears `MAIN`, `CALCS`, and `CALCULATOR` cache tables. This does not prove that every mutable build, UI, parser, or calculator-closure field is reset. Never share a live build or cached closure across workers.

The wrapper's startup error branch prints `promptMsg` and calls `io.read("*l")`; [Launch:OnFrame](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Launch.lua#L111-L122) also catches calculation errors into a prompt. A worker must convert these conditions into structured failures and terminate promptly instead of waiting for console input or returning stale outputs.

## Metric contract

These names are raw PoB output keys, not a proposed general API. Expose stable domain names in Rust with source-key provenance, unit, actor, selected skill, calculation mode, scenario, and explicit unavailable/non-finite status.

| Meaning | Raw field | Source and interpretation |
| --- | --- | --- |
| Effective elemental/chaos resistance | `FireResist`, `ColdResist`, `LightningResist`, `ChaosResist` | [CalcDefence.lua:946-970](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Modules/CalcDefence.lua#L946-L970) clamps total resistance between calculated min and max. Values are percentage points (`75`, not `0.75`). |
| Uncapped resistance and excess | `{Type}ResistTotal`, `{Type}ResistOverCap` | Same source; total is before clamping and excess is `max(0, total - max)`. `Missing{Type}Resist` is the shortfall from the calculated maximum. Do not assume an output key called `{Type}ResistMax` exists merely because similarly named modifiers do. |
| Effective hit pool | `TotalEHP` | [CalcDefence.lua:3392-3406](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Modules/CalcDefence.lua#L3392-L3406) multiplies average total survivable hits by configured incoming damage. It depends on the encounter, avoidance, mitigation, and enabled recovery assumptions; it is not life plus energy shield or a one-hit survival guarantee. |
| Maximum survivable hit by type | `PhysicalMaximumHitTaken`, `FireMaximumHitTaken`, `ColdMaximumHitTaken`, `LightningMaximumHitTaken`, `ChaosMaximumHitTaken` | [CalcDefence.lua:3778-3784](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Modules/CalcDefence.lua#L3778-L3784). Useful independent defensive constraints; retain the scenario and enabled temporary effects. |
| Main skill hit damage rate | `TotalDPS` | [CalcOffence.lua:4576](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Modules/CalcOffence.lua#L4576) combines average damage, speed, and multipliers. It is not the full build damage roll-up. |
| Main skill combined estimate | `CombinedDPS` | [CalcOffence.lua:6264-6266](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Modules/CalcOffence.lua#L6264-L6266) starts with `AverageDamage` when `skillData.showAverage` is true, otherwise `TotalDPS`, and later adds damage components. Verify the selected skill's semantics before labeling this as damage per second. Minion outputs are separately nested where present. |
| Configured build damage roll-up | `FullDPS`, `FullDotDPS`, `SkillDPS` | [Calcs.lua:292](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Modules/Calcs.lua#L292) selects groups through `includeInFullDPS`; [buildOutput](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Modules/Calcs.lua#L476-L482) exports the roll-up. Lock included skills and count/uptime assumptions in the request. |

For “resistances > 75%,” preserve the user's chosen comparison. `>= 75` means at least 75 effective resistance; `> 75` requires an effective value above 75 and therefore sufficient maximum resistance as well. “Capped” means no `Missing{Type}Resist`, which can differ from either condition. Name chaos separately rather than silently treating “elemental” as all four types.

Freeze external encounter and usage assumptions when comparing candidates: incoming damage mix, enemy level/type and speed, declared uptime assumptions, and skill targeting. Recompute candidate-derived buffs, resource/charge availability, reservation, and supporting-skill effects; a removed source must not leave its benefit enabled. The defence calculation reads [`enemyDamageType` and `EHPUnluckyWorstOf`](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Modules/CalcDefence.lua#L2206-L2224), as well as [`enemySpeed`](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Modules/CalcDefence.lua#L3410-L3415). These are part of the objective's meaning and cache key.

## Potential integration issues identified in source

1. **Pinned-source syntax concern:** [Main.lua:339](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Modules/Main.lua#L339) contains `count += 1`. The headless [`LoadModule` stub](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/_SimpleGraphic.def.lua#L437-L446) directly invokes `loadfile`; no source transformation was found in this bootstrap. This is an apparent incompatibility with standard LuaJIT syntax, even if `SaveModCache` is not called, because the module must parse. Reproduce before treating this pin as runnable. If needed, choose a verified passing revision or maintain an explicit, minimal adapter patch with its hash; do not hide a local vendor edit.
2. **Compression and timing are placeholders:** [`Deflate`, `Inflate`, and `GetTime`](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/_SimpleGraphic.def.lua#L362-L399) return empty strings or zero in headless mode. Raw XML avoids dependence on these compressed-share-code stubs. The adapter needs deliberate compression support and a monotonic clock; the Rust supervisor can measure elapsed evaluation time independently.
3. **Headless does not imply side-effect-free:** paths are stubbed, normal startup reads settings, and shutdown calls [`SaveSettings`](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Modules/Main.lua#L371). `REGENERATE_MOD_CACHE=1` enables [cache regeneration](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Modules/Main.lua#L123-L132). Workers need controlled paths/environment and no update/import network flow. Keep the submodule unchanged during evaluation.
4. **Stdout is not a clean protocol:** [`ConPrintf`](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/_SimpleGraphic.def.lua#L484-L486) uses `print`. Route diagnostic output away from the response stream before introducing JSON messages.
5. **Calculable does not necessarily mean legal:** comparison overrides can inspect isolated nodes, whereas allocation methods construct paths and dependencies. Keep connectivity, point budgets, weapon-set rules, skill validity, item compatibility, and resource constraints explicit. Verify them on the materialized candidate before reporting feasibility.

## Untested runtime hypotheses

A persistent worker should amortize data loading; several independent worker processes should permit parallel evaluations without sharing Lua globals. Hosting LuaJIT through `mlua` inside a Rust worker does not remove coordinator/worker IPC, native-module ABI, Lua lifetime, or global-state concerns. This investigation did not benchmark either hosting option.

Reloading baseline XML before every candidate should be safer than rolling back arbitrary tables, but it still requires isolation tests. Compare repeated A, A/B/A sequences, different evaluation orders, and separate fresh processes. Hash semantic candidate data rather than raw exported XML: `SaveDB` enumerates saver tables with `pairs`, so raw serialization order should not be assumed canonical.

## Follow-up validation

The [implementation document](implementation.md) owns the evaluator work sequence and
acceptance checklist: runtime boot, fresh-process calculation and export parity, mutation
and worker-isolation checks, and measured throughput and memory. Keep subsequent results
there so these source-inspection notes retain their original evidence boundary.

## User-supplied minion fixture

The [decoded reference fixture](../tests/fixtures/builds/pobarchives-Dfz36mCq.xml) is a
level 96 Sorceress / Disciple of Varashta export with `PathOfBuilding2` root and `0_5`
tree data. Its [metadata](../tests/fixtures/builds/pobarchives-Dfz36mCq.metadata.json)
records exact source/output hashes and bounded decoding checks. The investigation preserved
the two original exports and checked source structure and tree-ID presence. It did not
perform Lua evaluation or UI parity checks; see the implementation document for later results.

The selected group is Kelari, the Tainted Sands, with a minion-skill selector. All Full DPS
membership flags are literal `nil`; cached player FullDPS is zero. Three named skill
entries have no stable IDs, and manual/granted group provenance needs reconciliation.
These are import/metric-contract questions, not proof that the corresponding mechanics
are unsupported. Preserve raw input; resolve names, inclusion, actors, and duplication
through the pinned adapter before scoring. Never accept cached source values as fresh
calculation evidence or silently interpret unresolved entries as zero contribution.

This complex integration case complements small calibration cases. The target scope
includes required lists of multiple skills/items and joint class/ascendancy, tree, gear,
support-gem, and supporting-skill changes. Controlled per-dimension checks do not establish
coupled parity or search quality. Fixture coverage and acceptance tasks are tracked in the
[implementation document](implementation.md).
