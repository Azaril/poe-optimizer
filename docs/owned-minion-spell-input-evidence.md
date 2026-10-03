# Minion and spell occurrence input evidence

The complete pinned PoB source witnesses now establish concrete input consumers
for Original05's four skeletal summons, Frost Bomb and Ice Nova. The witnesses
below passed with JIT disabled and enabled. They support the next finite Import/data
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
Remove-Item Env:POE_ICE_NOVA_OCCURRENCE_SOURCE_CHILD -ErrorAction SilentlyContinue
cargo test -p poe-optimizer-pob --test owned_ice_nova_occurrence_inputs -- --ignored
Remove-Item Env:POE_SNIPER_ACTOR_ACTION_SOURCE_CHILD -ErrorAction SilentlyContinue
Remove-Item Env:POE_SKELETAL_ACTOR_ACTION_SOURCE_CHILD -ErrorAction SilentlyContinue
cargo test -p poe-optimizer-pob --test owned_sniper_actor_actions -- --ignored --test-threads=1
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
| `owned-ice-nova-occurrence-source-01` | 5,226,330 | `ef5fa7366c5c3c5fc98b31a947f52575dc9aebdf51333104a16aaa192dfea574` |
| `owned-sniper-actor-action-source-01` | 13,354,183 | `c854302d2d3516d20b09a4da02934bdc9eab854b6f71b53d6d3f7e2c9d67e005` |
| `owned-skeletal-actor-action-source-01` | 33,855,014 | `7a30005f1c59f0bde7344236beea633c6beaeae957d794db1538eb2adaf43474` |

These reports authenticate original
`tests/fixtures/builds/breadth-20260908/build-05.xml`, SHA256
`442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089`,
source manifest `8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675`
and catalog digest `b22849f6afaef20b49a578c2ed88314e014b893a71b7919c7b83ca95c6faa7ea`.
Reports retain pinned file hashes. Large reports are local evidence artifacts;
the tests and this document provide the durable reproduction path.

The additional `complete_ice_nova_occurrence_inputs_preserve_source_semantics`
test covers all five originals, seventeen input controls and a repeated original
across fresh load and two requested rebuilds, separately in both JIT modes.
Its twelve source pins cover the actual loader, count and full-DPS consumers.
Group count wins by presence, including zero; duplicate matching effects can
read the first matching copy's count. Full-DPS count and per-copy damage are
recorded separately. Independent action maps, disabled Gem/group controls and
archived-only changes retain physical identities and actual output availability.
These are finite source observations, not a claim that count or global switches
are universally inert. The witness does not choose the canonical parity lifecycle.

The `complete_sniper_actor_action_correspondence_preserves_source_selection`
witness covers all five originals and all six saved Sniper occurrences: one in
Original01 and five in Original05. Its 34 cases retain fresh load and two
requested rebuilds in both JIT modes. Each archived Original05 preset has a
separate activation observation followed by a fresh load with deliberate
MAIN Basic Attack / CALCS Gas Arrow selection. Controls cover missing, invalid
and independent actor/action settings, nested stat-set maps, duplicate keys,
duplicate physical copies, disabled sources and archived-only changes.

Activating an archived preset can remove stale generated groups and shift its
runtime indices. A bounded return hook on the original loader retains the exact
loaded group and Gem objects before that cleanup. It is removed before
calculation, and the requested JIT mode is verified. The observer records both
source and runtime group indices and never substitutes a same-name object.
No source method is replaced. The original calculations and output lifecycle
remain intact; the witness does not select a canonical parity lifecycle.

The source proves a singleton Sniper actor and two distinct child actions. MAIN
uses `skillMinion`; CALCS uses `skillMinionCalcs` only for its main summon.
CALCS child-action selection always uses its own saved child index. Gas Arrow's
three stat sets remain distinct from Basic Attack's singleton set. These facts
support the bounded [Import correspondence](owned-source-actions.md); source
fallbacks and duplicate overwrites do not authorize permissive native import.

## Three-family actor/action breadth

The additional
`skeletal_families::complete_skeletal_families_preserve_actor_action_correspondence`
test uses the same lifecycle harness, original-loader hook and Lua observer as
the Sniper test. Its [test-only family table](../crates/poe-optimizer-pob/tests/support/skeletal_actor_families.rs)
selects the physical sources to observe; actual actor, child and stat-set objects
still come from the unchanged source runtime.

The 62 cases retain all five unchanged originals, five Original05 preset
activation observations, twelve separate focused physical occurrences, thirteen
controls for each family, and a repeated Original05. They cover all fifteen
original physical copies: six Arsonists, four Frost Mages and five Reavers,
distributed across Original01 (three) and Original05 (twelve). Every case records
fresh load and two requested rebuilds in both JIT modes, with exact source and
runtime object joins and thirteen pinned source files.

Controls independently change MAIN/CALCS actor, child and stat-set selectors,
including absent, invalid and clamped settings, duplicate map keys and physical
copies, disabled Gems/groups and archived-only edits. The observer retains actual
action availability. Archived activation and deliberate subsequent focus remain
separate observations, preserving the loader's group removal and index shifts.
All five first-child constructed stat sets use `skill_stat_descriptions`:
Arsonist's Fire Bomb/Hidden, Frost Mage's Projectile/Explosion and Reaver's Basic
Attack. Their second children are observed separately; unresolved catalog Command
references remain unresolved.

Both tests passed together in **398.71 seconds**, serialized with
`--test-threads=1`. The new 62-case JIT reports are byte-identical, and the older
34-case Sniper report retains its historical bytes and digest listed above.
Leave **both** actor/action child variables unset when reproducing this combined
gate. The evidence establishes finite source topology and selection correspondence;
publication of the three-family native data and input inventories remains a
separate gate. It does not establish native damage, final-input or whole-build
parity, or select a canonical parity lifecycle.

## Next Import/data consumers and limits

The [skill-preset usage contract](owned-skill-usage-proposal.md) now has Boolean
and numeric Import projection. Sniper count and its ordinary native reservation
component are implemented; complete usage inventories, real final-input and
modifier producers remain unfinished. See the [reservation contract](owned-summon-reservation.md).

Ice Nova now has checked primary supply and two owned action alternatives.
The [source-action adapter](owned-source-actions.md) converts exact physical
MAIN/CALCS selections into existing query targets. The additive V3 physical
disposition accounts for intrinsic inputs, reviewed reference settings and
deferred usage attached to the actual preset's Pending inventory. It does not
invent a Boolean consumer or erase metadata aliases. The older physical policies
retain their contracts; complete usage and numerical mechanics remain unfinished.

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
