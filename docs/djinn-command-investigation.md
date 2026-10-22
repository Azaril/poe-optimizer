# Djinn Command source and execution gaps

Investigated October 20, 2026. This records evidence and remaining work, not an
approved new public model. The [implementation plan](https://github.com/Azaril/poe-optimizer/blob/fa49fcea32f0413cde5f5983e179c34e6c7acbdf/docs/implementation.md#current-checkpoint-configuration-admission)
controls delivery; [resource obligations](owned-resource-obligations-proposal.md)
already define the accepted direction for sustained costs.

## Findings

Kelari's Deception (passive 23265) and Navira's Fracturing (10561) describe
Player-granted Commands. These are distinct from the abilities executed by the
existing Sand and Water Djinn. The pinned runtime data omits the Player Commands:
`CommandSandDjinnExplosiveTeleportPlayer` and
`CommandWaterDjinnGroundBurstPlayer`. Its grant-name parser therefore cannot
resolve those passive lines. The minion profiles independently list the damage
abilities, and PoB permits selecting them with either unlock unallocated.

The source is PoB revision `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`,
with source manifest
`8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675`.
Relevant complete runtime boundaries are `Modules/ModParser.lua:2183–2200`,
`Data/Gems.lua:9181–9231`, `Data/Minions.lua:998–1029,1098–1127` and
`Modules/CalcActiveSkill.lua:1116–1185`. The retained source manifest authenticates
the Lua files. `Export/Skills/SkillGems.txt` lists the missing Commands under
“Potentially Disabled Other”; `SkillGemsExport.txt` has empty records. These two
text files are locally hashed corroboration only: they are outside that manifest,
and independent pinned-Git-object authentication was unavailable.

Individual current [Kelari](https://poe2db.tw/us/Kelaris_Deception) and
[Navira](https://poe2db.tw/us/Naviras_Fracturing) descriptions corroborate the
Player/minion distinction. They are not version-aligned numerical authority.
No bulk extraction was performed.

## Controls and limits

Four immutable derivatives of Original05 retain both Barya suppliers and differ
only in the third passive preset's two unlock allocations: both present, either
one removed, and both removed. Every input has an exact byte inverse. Each was
evaluated for the tree-supplied and manual occurrences of both Djinn, giving
sixteen evaluations. Eight independent fresh processes repeated both-present
and both-absent cases.

All corresponding compared results match exactly: backend identity, selected
Player/minion identities, group census, typed measurement, raw Player/minion
metric maps and context. Replays also preserve the build summary. Input identity
and allocation census intentionally differ for the removal controls. These are
ordinary MAIN evaluations, not a new cold/rebuild or paired-JIT lifecycle claim.
The earlier provider witness separately authenticates identical JIT-off/on
reports and distinct manual versus allocation-supplied occurrences.

This establishes a deterministic reference coverage limitation. It does not
establish legal gameplay execution without the unlock, native numerical parity
or a complete build. Keep both passive inventories and execution coverage
incomplete; do not make the child ability unconditional or introduce a numerical
exception on this evidence.

Local evidence is in `runs/djinn-unlock-audit-20261020/`. Commitments:

| Artifact | SHA-256 |
| --- | --- |
| Original05 immutable XML | `442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089` |
| `findings.json` | `bc6d6ca90baa9cd1a36660141185304b866e83f655daf547adc127601819bf6e` |
| `controls-input-proof.json` | `e5cb23f61de345e4e0f4f36e827a83d32eab83972351e59e04a7e51e04925b7e` |
| Existing provider witness, each JIT mode | `6c53dc1e853331f4a7d2cdfee60d9b0b04b92ec814a673185456300a11cbc916` |

Independent review verified input inverses, report comparisons and retained
provider evidence. These local diagnostic reports are not portable CI fixtures.
Any eventual production closure needs its own retained source evidence and
ordinary Rust regression tests.

## Next work and design boundary

First obtain versioned Command metadata and evidence for recipient selection,
participation and lifecycle. A newer game snapshot must be reconciled explicitly
against retained data; the PoB application version and `0_5` tree label do not
identify an exact game-client snapshot. Deception also describes a temporary
clone and a separate later explosion. Preserve that unresolved population and
timing behavior rather than assuming a direct one-to-one invocation.

Existing passive grants can express the positive Player Skill identity. Existing
Skill/Actor/Action identities and provider paths can distinguish both endpoints.
The new question is execution authority: which exact source may invoke which
exact recipient, under which participation and usage conditions? Original05 has
manual and tree-supplied Djinn simultaneously, so matching a definition and taking
the first Actor is invalid.

Before adding public types, reconcile a bounded execution relationship with the
accepted Action-dependency work. It must specify one/all/explicit recipients,
missing or ambiguous targets, permitted value dependencies, execution frequency,
costs, cooldowns and cycle handling. Contrast Player Command → minion ability,
trigger → payload Action and companion control → ability. A command's per-use
Player cost is distinct from sustained Skill reservation and from the recipient's
own timing and modifiers. Disabling control must not disable unrelated autonomous
abilities.

Reuse preparation/participation, exact identities, checked recipient validation
and native graph planning. The current `PayloadLink` only addresses authored
Skill uses, and Engine rejects nonempty payload relations; it is not an implemented
dispatch mechanism. Exact Action contribution queries do not grant cross-Action
inheritance, and effect-application delivery alone does not authorize execution.
Present a concrete proposal for review before changing this public contract.
