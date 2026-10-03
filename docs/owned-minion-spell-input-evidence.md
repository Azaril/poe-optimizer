# Minion and spell occurrence input evidence

The complete pinned PoB source witnesses now establish concrete input consumers
for Original05's four skeletal summons, Frost Bomb and Ice Nova. Both witnesses
passed with JIT disabled and enabled. They support the next finite Import/data
conversions; they do **not** establish complete native input inventories, action
coverage, numerical parity or a working original build. Complete native builds
remain **0/5**.

These are optional reference tests running the unchanged source lifecycle, with
source-function authentication and preservation checks. PoB UI state and Lua
objects do not become part of the native evaluator contract.

## Minion usage and action correspondence

The [minion witness](../crates/poe-optimizer-pob/tests/owned_minion_occurrence_inputs.rs)
observes 68 controls across Arsonist, Sniper, Frost Mage and Reaver, plus original,
repeated and warm-restored builds. In the tested frame all four summons have
HasReservation and MultipleReservation, with zero free Spirit count. Count has a
real reservation consumer:

| Families | Spirit at count 1 | Gem count 3 | Group count 4 overrides Gem 3 | Group count 0 overrides Gem 3 |
| --- | ---: | ---: | ---: | --- |
| Arsonist, Frost Mage | 51 | 153 | 204 | Per-skill reservation field absent |
| Sniper, Reaver | 39 | 117 | 156 | Per-skill reservation field absent |

These numbers are observations, not coefficients to embed in native code.
`CalcDefence.lua` rounds ordinary reservation before multiplying its flat
component by `max(count - free_count, 0)`. Actor type, child action and per-actor
DPS remain unchanged by count-only controls, including zero. Count does not mean
additional physical Gems or newly materialized actor records.

FullDPS inclusion is separate. It scales counted direct damage but applies other
aggregation rules to ailments. Arsonist's count-three control includes three
direct-DPS contributions and **one** best Ignite contribution. Multiplying its
entire CombinedDPS by three would be wrong.

Both global switches are inert for all four current primaries: they are non-Vaal,
have `has_global_effect=false`, and toggling either switch preserves the observed
summon, reservation and outputs. UmbralWell is inactive in the tested contexts.
Do not copy Pain Offering's global activation meaning onto these summons.

Each actor has an exact singleton minion choice and two resolved child abilities:

| Actor | Child 1: constructed stat sets | Child 2: constructed stat sets |
| --- | --- | --- |
| RaisedSkeletonArsonist | FireBombSkeletonMinion: Fire Bomb, Hidden | DestructiveLinkSkeletonBombadierMinion: Explosive Demise |
| RaisedSkeletonSniper | MinionMeleeBow: Basic Attack | GasShotSkeletonSniperMinion: Impact, Poison Cloud, Explosion |
| RaisedSkeletonFrostMage | FrostBoltSkeletonMageMinion: Projectile, Explosion | IceArmourSkeletonMageMinion: Ice Armour |
| RaisedSkeletonReaver | MinionMeleeStep: Basic Attack | EnrageSkeletonReaverMinion: Enrage |

The listed stat sets are in observed source index order. Every child has an exact
summon/actor relation and **no physical source instance**. The missing Command
references in the Gem catalog remain unresolved; these child abilities do not
resolve or replace them.

Original MAIN selects Sniper/group 3/Basic Attack; CALCS selects Arsonist/group
1/Fire Bomb. CALCS uses the alternate actor-name field only for its main summon.
Controls independently select the second child and alternate constructed stat
sets in each context. Missing/invalid actor names resolve to the singleton actor
in this finite frame; that does not authorize a general unknown-name fallback.
MAIN/CALCS are source reference contexts, not new game-state fields for Core.

## Constructed spell stat sets

The [spell witness](../crates/poe-optimizer-pob/tests/owned_spell_stat_set_inputs.rs)
records a baseline, 13 controls and a repeated baseline. It enumerates actual
constructed tables and compares selected table identity separately in MAIN and
CALCS.

- Frost Bomb has one table, “Frost Bomb”, and `has_global_effect=true`.
- Ice Nova has **two** tables, “Ice Nova” and “Cold-Infused”. Its two metadata
  aliases, `IceNovaPlayerOnFrostbolt` and `IceNovaColdInfusedPlayer`, are neither
  standalone Skills nor additional physical effects. They do not imply three
  tables or an alias-to-index mapping.
- Both valid Ice Nova indices work independently across contexts and duplicate
  physical occurrences. Legacy scalar headers are overwritten; unrelated keys
  and missing/malformed indices select the observed default. Duplicate child
  rows use the last source entry in these controls.
- Numeric indices 0, -1, 1.5 and 3 fail the complete source lifecycle. They are
  not clamped to table 1. Native admission may conservatively keep malformed or
  duplicate selections Pending rather than reproduce malformed source behavior.

Skill Part and stat-set selection remain separate. Source labels and ordinals
need explicit owned ActionOutput/Part/Mode/StatSet correspondence before use.

## Reproduce and identify the evidence

Run from the repository root with the complete pinned
`vendor/path-of-building-poe2` checkout available. These tests are ignored by
default. Leave the child-mode variables unset so the parent tests run **both**
JIT modes and compare them. Their subprocesses and LuaJIT are reference-test
infrastructure; native evaluation does not acquire those dependencies.

```powershell
Remove-Item Env:POE_MINION_OCCURRENCE_CHILD -ErrorAction SilentlyContinue
Remove-Item Env:POE_SPELL_STAT_SET_SOURCE_CHILD -ErrorAction SilentlyContinue
$env:CARGO_PROFILE_TEST_OPT_LEVEL = '2'
$env:CARGO_PROFILE_TEST_DEBUG_ASSERTIONS = 'true'
$env:CARGO_PROFILE_TEST_OVERFLOW_CHECKS = 'true'
cargo test -p poe-optimizer-pob --test owned_minion_occurrence_inputs -- --ignored
cargo test -p poe-optimizer-pob --test owned_spell_stat_set_inputs -- --ignored
```

The exact tests are
`complete_source_minion_occurrence_settings_keep_exact_actor_actions` and
`complete_source_spell_stat_sets_preserve_constructed_correspondence`. Each child
has a 300-second deadline. For a single-mode diagnostic only, set its named
child variable to `off` or `on`; that bypasses the parent comparison and is not a
two-mode validation.

Each directory contains `source-jit-off.json`, `source-jit-on.json` and logs.
The two JSON files in each pair are byte-identical at this checkpoint:

| Directory under `runs/` | Bytes per JSON | SHA256 per JSON |
| --- | ---: | --- |
| `owned-minion-occurrence-inputs-01` | 25,157,026 | `1b07bd0cc1b4aa6166189064ecece004f1a4068f5a985a64d3bd61e07a5bbc86` |
| `owned-spell-stat-set-source-01` | 2,236,787 | `3da1b32a4ed08336ea603b5c85190252ab63aa95e3d40ebf5d5c8869eecd2326` |

Both authenticate original
`tests/fixtures/builds/breadth-20260908/build-05.xml`, SHA256
`442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089`,
source manifest `8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675`
and catalog digest `b22849f6afaef20b49a578c2ed88314e014b893a71b7919c7b83ca95c6faa7ea`.
Reports retain pinned file hashes. Large reports are local evidence artifacts;
the tests and this document provide the durable reproduction path.

## Next Import/data consumers and limits

The [skill-preset usage contract](owned-skill-usage-proposal.md) now has Boolean
and numeric Import projection. Sniper count and its ordinary native reservation
component are implemented; complete usage inventories, real final-input and
modifier producers remain unfinished. See the [reservation contract](owned-summon-reservation.md).

Ice Nova now has checked primary supply and two owned action alternatives.
The [source-action adapter](owned-source-actions.md) converts exact physical
MAIN/CALCS selections into existing query targets while preserving unresolved
physical and usage inventories. The existing physical-inventory proof still
assumes a global-effect Boolean; the next checkpoint must separate intrinsic
inputs from reference settings and actual deferred usage obligations. Do not
invent a Boolean consumer or erase metadata aliases to satisfy that proof.

Reuse existing actor-owned supply and action declarations for the observed
skeletal child inventories, preserving all original queries and unresolved
Commands. Neither topology nor component parity grants complete action or
physical-input inventory authority.

Unknown child maps, incomplete destinations, alternative reservations, missing
Commands, supported final inputs and unproved modifier inventories remain
unresolved. See [physical input boundaries](owned-gem-inputs.md) and
[effective inputs](owned-effective-gem-inputs.md). These witnesses provide
specific source evidence; native integration and whole-build parity are separate
acceptance gates.
