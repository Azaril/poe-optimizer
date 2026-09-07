# Skill and tree coverage

This document records evaluator coverage at PoB commit `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`. The [fixture XML](../tests/fixtures/builds/pobarchives-Dfz36mCq.xml) remains the exact decoded user input. Coverage describes what the fresh MAIN evaluation resolved and selected; it does not certify game accuracy, build legality, or every mechanic interaction.

## Output contract

[`BuildCoverage`](../crates/poe-optimizer-core/src/coverage.rs) is extracted by [`coverage.lua`](../crates/poe-optimizer-pob/src/coverage.lua) after MAIN calculation. It records:

- The active skill set, each runtime group and gem, IDs, enabled state, level, quality, counts, catalog resolution, and PoB's unresolved-entry diagnostic.
- Group provenance from actual `sourceItem` and `sourceNode` objects, plus the raw `source` string. A source string without either object is `other_generated`. A manual entry for a catalog effect marked `fromTree` remains manual.
- The selected player and minion actions, their owning group/gem, action ID, actor action-list index, part, stat set, minion ID, and final `show_average` flag (after offence calculation, not the catalog default). A minion action belongs to its summoning entry; it is not itself an imported gem.
- Full DPS group membership, MAIN active-skill count metadata, and the rows actually returned in PoB's `SkillDPS` aggregate. The adapter does not sum independent MAIN outputs or deduplicate overlapping groups.
- Connections to absent nodes in every loaded tree, with source and target IDs and whether the endpoints are allocated in the current build's tree.

All group/gem/action positions are one-based and refer to the evaluated active skill set. PoB can generate, remove, clamp, or normalize entries during loading and calculation; these positions and normalized IDs are not lossless import identities. `gem_id` is the evaluator's gem-catalog key, while `gem_game_id` corresponds to the game's gem ID. Their values are versioned by the runtime/source fingerprint. The preserved input XML remains authoritative for original attributes and unknown data.

Resolution is `resolved_gem`, `resolved_granted_effect`, `unresolved`, or `empty`. Empty editor slots are distinguished from unresolved requested skills. A related catalog candidate is a review hint, not a repair. Exact display-name matching deliberately does not guess identity across renamed or localized skills. Counts and contribution values are nullable when absent or non-finite; `included` and `count_enabled` describe PoB inputs, not proof of positive damage contribution.

## Supplied fixture findings

The fixture has 19 runtime skill groups, including 15 manual groups, three tree-granted groups, and one item-granted group. There are exactly three unresolved entries:

| Group / gem | Imported name | Observed relationship | Required interpretation |
| --- | --- | --- | --- |
| 6 / 1 | Spectre: Powered Zealot | Two exact monster-name matches: `Metadata/Monsters/VaalMonsters/Zealots/VaalZealotSpearLightning` and `Metadata/Monsters/VaalMonsters/Zealots/VaalZealotDaggersLightning` | Ambiguous Spectre identity. Preserve both candidates; choosing a monster requires an explicit repair or a verified richer source. |
| 13 / 1 | Navira's Well | Minion action `ESRechargeForceRestartWaterDjinn`, catalog action 3 of `WaterDjinn` | This is a minion command, not a standalone named gem. The action describes energy-shield recharge and carries `base_deal_no_damage`; its existence does not verify that every intended buff interaction is modeled. |
| 14 / 1 | Kelari's Deception | Minion action `ExplosiveTeleportSandDjinn`, catalog action 2 of `SandDjinn` | The same action is already selected through the main summoning entry, group 1 / gem 1. That does not resolve this separate imported entry or prove it should be deleted. |

PoB resolves the main player action to `SummonSandDjinnPlayer` and its selected minion action to `ExplosiveTeleportSandDjinn`. The catalog declares `base_skill_show_average_damage_instead_of_dps`, but the final MAIN `show_average` value is **false**. PoB maps that stat to `skillData.showAverage`, classifies this non-triggered minion spell as `selfCast`, then applies its cooldown to speed and explicitly clears average mode (`CalcOffence.lua:3005-3014`). The coverage field reports this final decision; it must not be forced true from the catalog declaration.

A fresh diagnostic evaluation on 2026-09-07 confirmed stat set 1, average damage `111001.25770786172`, cooldown `1.8382352941176472` seconds, speed `0.544` per second, and TotalDPS `60384.68419307677`. Here TotalDPS equals average damage multiplied by the cooldown-limited speed. This verifies the reason for the mode flag and the adapter's observation, not independent game-mechanic parity.

Under the current typed-metric policy, `selected_hit_dps` can expose this selected actor/action rate when hit damage exists and final average mode is false. Average mode true makes that metric unavailable until usage semantics are defined; `selected_average_hit` retains damage units. Neither state certifies sustainable build damage for commands, charges, clones, or action scheduling. A stricter future mechanic-coverage policy should express those unknowns separately rather than changing the observed PoB flag. `CombinedDPS` remains an opaque attachment field: upstream switches its base between AverageDamage and TotalDPS according to the final mode.

Generated groups preserve these identities:

| Group | Provenance | Skill |
| --- | --- | --- |
| 16 | Tree node 13289 | `SummonSandDjinnPlayer` |
| 17 | Tree node 34207 | `SummonFireDjinnPlayer` |
| 18 | Tree node 32705 | `SummonWaterDjinnPlayer` |
| 19 | Item 9, Weapon 1, Morbid Roar / Rattling Sceptre | `SummonSkeletalWarriorsPlayer` |

Manual groups also contain these effects with different levels, supports, or selected actions. Identity overlap is visible for review and future canonicalization; automatic suppression would require verified grouping semantics. Every fixture `includeInFullDPS` input is the literal string `nil`, which PoB loads as false. No group is included and the returned Full DPS aggregate is zero. This does not establish zero build damage.

## Startup tree topology

The 14 `missing node` messages occur while loading the latest tree (`0_5`) at startup, before the fixture is imported. The raw tree has the following class-start connections to absent targets:

| Source node / classes | Missing targets |
| --- | --- |
| 44683 / Shadow, Monk | 5162, 45406, 50198 |
| 47175 / Marauder, Warrior | 16732, 51916, 54579 |
| 50459 / Ranger, Huntress | 24665 |
| 50986 / Duelist, Mercenary | 39383, 10889, 62386 |
| 61525 / Templar, Druid | 35715, 26353, 950, 28429 |

PoB skips each missing target before adding the corresponding `linkedId` adjacency. The messages therefore concern global tree topology as well as connector drawing. All affected starts retain some valid neighbors. The Witch/Sorceress start has none of these dangling connections; none of the 14 absent targets is allocated in the supplied Sorceress build. That does not establish the safety of searching other classes on this tree revision.

Coverage discovers these relationships from loaded `tree.nodes[*].connections[*].id` and node IDs, without parsing stderr or hardcoding this list. A connection's absent target remains a coverage issue even when `missing_target_allocated` is false. Its optimization impact remains unverified until compared with corrected upstream/game data. Ordinary stderr is also retained in the evaluation snapshot.

## Source evidence

The following pinned upstream sources define the behavior used by the extractor:

- [`SkillsTab:LoadSkill`, `ProcessSocketGroup`, `FindSkillGem`, and `Save`](../vendor/path-of-building-poe2/src/Classes/SkillsTab.lua): input normalization, catalog resolution, selected-action attributes, and export identities. `FindSkillGem` searches the gem catalog; a minion-action display name does not thereby resolve as a gem.
- [`CalcSetup`](../vendor/path-of-building-poe2/src/Modules/CalcSetup.lua), granted-skill processing around lines 1671-1730: item/tree objects attached to generated groups; fallback unarmed action around lines 2196-2207.
- [`CalcActiveSkill`](../vendor/path-of-building-poe2/src/Modules/CalcActiveSkill.lua), `createActiveSkill` and `createMinionSkills`: summoning ownership, stat sets, parts, minion skill-list filtering, and clamping. Catalog action positions can differ from evaluated action positions.
- [`Data/Minions.lua`](../vendor/path-of-building-poe2/src/Data/Minions.lua), `WaterDjinn` and `SandDjinn`; [`Data/Skills/minion.lua`](../vendor/path-of-building-poe2/src/Data/Skills/minion.lua), `ESRechargeForceRestartWaterDjinn` and `ExplosiveTeleportSandDjinn`: the two command/action relationships and their declared effects.
- [`Data/Spectres.lua`](../vendor/path-of-building-poe2/src/Data/Spectres.lua), the spear and dagger Powered Zealot records; [`Data/Gems.lua`](../vendor/path-of-building-poe2/src/Data/Gems.lua), `Metadata/Items/Gems/SkillGemSummonSpectre`: distinct monster identities and the generic `Spectre: {0}` gem wrapper.
- [`SkillStatMap.lua`](../vendor/path-of-building-poe2/src/Data/SkillStatMap.lua), average-damage stat mapping, and [`CalcOffence.lua`](../vendor/path-of-building-poe2/src/Modules/CalcOffence.lua), cooldown/rate processing at lines 3005-3014 and TotalDPS at lines 4564-4576: final average mode can differ from the catalog declaration.
- [`calcs.getActiveSkillCount`](../vendor/path-of-building-poe2/src/Modules/CalcDefence.lua) and [`calcs.calcFullDPS`](../vendor/path-of-building-poe2/src/Modules/Calcs.lua): group/gem count precedence, eligibility, and aggregate contributions.
- [`Main:LoadTree`](../vendor/path-of-building-poe2/src/Modules/Main.lua), [`GameVersions.lua`](../vendor/path-of-building-poe2/src/GameVersions.lua), [`PassiveTree` connection processing](../vendor/path-of-building-poe2/src/Classes/PassiveTree.lua) around lines 307-327, and [`TreeData/0_5/tree.lua`](../vendor/path-of-building-poe2/src/TreeData/0_5/tree.lua): eager startup loading and skipped missing-target edges.

## Remaining validation

Runtime integration tests should assert the three classifications, ambiguous Spectre candidates, selected minion ownership and final `show_average = false`, manual/generated provenance, absent Full DPS membership, and 14 dangling connections. Selection changes must update the context while preserving unresolved entries. Additional fixtures should exercise enabled Full DPS groups, empty editor slots, disabled skills, synthesized fallback attacks, and non-minion builds. Independent numeric parity and complete mechanic/legality coverage remain separate work tracked in [implementation.md](implementation.md).
