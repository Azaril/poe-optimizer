# Explicit Cleric and Heal topology

This migration adds the typed supply chain for the pinned Skeletal Cleric and its
Heal ability. It uses the existing generic actor-supply model and a distinct Cleric
Actor definition. Runtime evaluation reads owned schemas and rules; this data does
not introduce a source-language callback or skill-name dispatch.

The checked predecessor is the full actor-ability release
`36b82e2924aa5a34cd2a35d01d395d462ea637527e24b39f64d84cf2a0deb07f`.
This is a version 1 structural migration with schema 3 and operations 11. It has no
evaluation-artifact group. A later support release must author and bind that group
explicitly through the version 2 migration contract.

## Typed supply chain

Existing Gem `0916`, summon Skill `0325`, and Heal Skill `01c6` retain their IDs.
The 13 append-only allocations below extend the predecessor's last ID, `309b`.
Every abbreviated ID has the prefix `def.000000000000`.

| ID | Declaration | Meaning |
| --- | --- | --- |
| `309c` | Actor | Skeletal Cleric population definition |
| `309d` | Gem `0916` | Supply summon Skill `0325` |
| `309e` | Gem `0916` | Activate the summon supply |
| `309f`, `30a0` | Skill `0325` | Required effective summon level and quality |
| `30a1` | Skill `0325` | Population slot using Actor `309c` |
| `30a2` | Skill `0325` | Activate the Cleric population |
| `30a3` | Actor `309c` | Supply Heal Skill `01c6` |
| `30a4` | Actor `309c` | Activate the Heal supply |
| `30a5`, `30a6`, `30a7` | Skill `01c6` | Required effect level, quality, and actor level |
| `30a8` | Skill `01c6` | Heal output on the exact provider Actor |

The Heal provider path contains the Gem grant, population grant, and Actor ability
grant, in that order. Its Actor identity uses the summon provider and Cleric
population slot. [bindings.json](bindings.json) supplies these exact typed addresses
for authoring and tests. The ordinary part/mode/stat-set descriptors are shared
definitions; the Heal output and provider path are distinct from Sniper's.

## Known facts and retained gaps

The four rule owners remain Partial. Gem and summon programs activate their exact
grants. The Actor program supplies Heal effect level 1, quality 0 percentage points,
and the current exact Actor's level stat as three separate required parameters.
Existing parent activation and required-input gates still apply.

There is deliberately no producer for effective summon level/quality or Cleric actor
level in this migration. Physical Gem level 19 and quality 20 from original01 do not
prove those effective values. Missing values stay missing. The actor-level input
accepts the existing reviewed 1–100 range without clamping; further range support
requires evidence. Neither Sniper's level tables nor its quality effects are reused.

The original Gem input members and Partial gaps remain. The original Unmapped Skill
input gaps are retained within their new Partial declaration collections. Heal rules
remain explicitly unconverted. The built-in population lists Heal, while population
membership and Actor declarations retain an `extra-minion-skills-unconverted` gap.
The source can add abilities through `ExtraMinionSkill`; a built-in list cannot prove
complete whole-build ability membership.

Actor direct parameter and socket collections are Complete and empty, as required
by the accepted actor-supply contract. Actor facts enter through typed stats and
ability parameters through explicit projections; this model has no consumer for
direct Actor parameters or sockets. That structural restriction does not close
the Partial ability membership or whole-owner behavior coverage.

Cleric Actor-provider choices are also Complete and empty, based on the pinned
constructor. `RaisedSkeletonCleric` has fixed population data, and the summoning
effect declares neither `minionHasItemSet` nor `minionUses`. The constructor reads
`skillMinion` from the parent skill source to select the population; this effect's
list contains only Cleric. Later `skillMinionSkill` selects a reported child action
after the children exist, and stat-set selections belong to the child/action
context. None is an independent choice on this Actor provider. This does not close
the parent or Heal choice collections or the `ExtraMinionSkill` membership gap.

No healing amount, damage, reservation, support transfer, or complete build result is
claimed. This checkpoint establishes explicit topology; whole-build native parity
remains gated by the incomplete schemas and rules.

## Source evidence and query preservation

The source is pinned to `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`:

- `src/Data/Skills/act_int.lua` declares `SummonSkeletalClericsPlayer` and its
  `RaisedSkeletonCleric` population.
- `src/Data/Minions.lua:245` lists Heal, Resurrect, and DoLiterallyNothing. Only
  `HealSkeletonClericMinion` has a definition in this pinned skill catalogue.
- `src/Modules/CalcActiveSkill.lua` filters absent skill definitions when creating
  child abilities and constructs Heal with effect level 1 and quality 0.
- `src/Data/Skills/minion.lua:671` has one outer Heal effect level and separate
  stat-set rows carrying actor-level labels. The observed interpolation mode 1
  selects effect-level row 1: raw regeneration remains 776 at actor levels
  24/60/62/80 in the reference probes. Carrying a distinct actor-level input does
  not establish that this Heal stat scales with it.

The absent Resurrect and DoLiterallyNothing entries are recorded source evidence,
not invented native abilities. See the scoped [support evidence](../support-release/README.md)
and [actor-supply design](../../../../../docs/owned-actor-skill-supply.md).

All five originals and all 110 query rows are preserved byte for byte. In
`tests/fixtures/builds/breadth-20260908/build-01.xml`, line 3 selects main socket group
1; its Gem at line 149 is Sand Djinn. Cleric is enabled group 5, at line 179. The
existing unresolved selected-minion queries cannot be reassigned to Heal. This
migration therefore has an empty `query_targets` list.

Publish into a new directory from the exact checked predecessor:

```text
poe-optimizer assemble-owned-release runs/owned-actor-abilities-01/package --migration data/owned/poe2/3887ae68/cleric-ability-supply/migration.json --output runs/owned-cleric-abilities-02/package
```

The living [implementation document](../../../../../docs/implementation.md) records
validation and publication status. Source witnesses and structural tests do not
remove the retained coverage gaps.

The normal `owned_extension_cli` test rebuilds the release chain and checks this
data against all five original imports. For a focused local reproduction after
publishing the checked release, its Rust native regression can reuse that directory:

```powershell
$env:POE_OPTIMIZER_TEST_CLERIC_RELEASE = 'C:\code\poe-optimizer\runs\owned-cleric-abilities-02\package'
cargo test -p poe-optimizer-cli --test owned_extension_cli cleric_native::published_cleric_release_native_regression -- --ignored --exact
```

It loads the full published schema and stored rules without changing their coverage.
Generated Heal Skills resolve to distinct Actor occurrences, while full Actions remain
unresolved on the unconverted Heal choice schema. Exact component projections, absent
effective-input producers, disabled/sibling binding and serial/Rayon agreement are
checked separately; this reproduction does not replace the full release-chain test.
