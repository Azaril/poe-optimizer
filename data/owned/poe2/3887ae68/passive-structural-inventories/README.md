# Intrinsic passive structural inventories

This historical offline review changed 18 direct inventory closures on six existing
passive definitions. It adds no definitions, members, numerical rules, source
domains or runtime branches. The predecessor was input
`35d446c73be3aa977d55a49174aaf4656e27a38ebaeaab354e86cd7f9f36b5a5`.
The retained migration used the then-current schema-7, migration-5,
operations-28 contract. It is evidence of the reviewed delta, not the current
publication workflow.

| Source node | Existing owned definition | Reviewed direct declarations |
| --- | --- | --- |
| 13289, Barya of Kelari | `0b34` | Grant `32d3` supplies `32d2`, whose Skill is `0322`; no direct Actor |
| 19318, Puppet Master effect | `0d0f` | Empty grants, Actors and skill supplies |
| 32705, Barya of Navira | `10d4` | Grant `32d5` supplies `32d4`, whose Skill is `032c`; no direct Actor |
| 35560, At your Command | `11c2` | Empty grants, Actors and skill supplies |
| 45333, Archon Effect | `14d8` | Empty grants, Actors and skill supplies |
| 62210, command efficiency and Puppet capacity | `19ec` | Empty grants, Actors and skill supplies |

All IDs in this table abbreviate existing `def.000000000000....` keys.
The two positive Barya chains are preserved in full. Their summon Skills own
their descendants; descendant and numerical coverage remain Partial. The
other four nodes modify existing recipients, costs or buff capacity. This
does not establish that their calculations or population/lifecycle behavior
are implemented.

`dependencies.json` retains exact predecessor descriptors, rule owners,
positive-chain slots, descendant Skills and all six unresolved descriptors.
`migration.json` only replaced the six definitions. The retained proof restores
the three closures on each replacement and requires exact descriptor equality.
The historical whole-release publisher separately authenticated the actual
predecessor and preserved every other schema and recipe field.

`evidence.json` joins all twelve candidate nodes to the complete retained
tree, selected bindings, parsed recipes and source identities. Nine bounded
full source archives and sixteen reviewed spans cover intrinsic node
construction, complete stat processing, grant factories, class-start
injection, and the separation of external jewels, mastery choices and node
replacements. Generated stat-description records account for all five
non-grant stat lines, including four parser remainders. A parser rejection
or empty parsed list never proves that a modifier is inert. The exact
source-to-Skill identity and retained `ExtraSkill` grant recipe account for
each of the two remaining lines.

The source revision is
`3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`. Lua source bodies are authenticated
against the retained source manifest. The full tree JSON has its own exact
digest, also retained by the earlier Djinn-grant evidence. `authoring.json`
seals these artifacts and the upstream license; the Rust checker pins its
exact bytes independently. Ordinary tests need no vendor checkout, Lua,
network access or generated `runs/` package.

The census is complete for the twelve candidates; structural closure is not.
Nodes 14686, 26926, 53324 and 53795 can grant Puppet Master or Archon of Undeath
buffs. Their complete, version-aligned buff payloads are absent from the
retained exports, so their direct inventories remain Partial. Nodes 10561
and 23265 describe separate Djinn Command grants and also remain unresolved.
None of these six nodes is treated as an empty declaration.

The next buff evidence needs a game-content snapshot with explicit provenance,
the relevant BuffDefinitions/BuffTemplates records and their referenced
Stats/descriptions, followed by a complete payload review. The pinned export
code describes those table layouts but does not retain their raw records.
The tree label `0_5` does not identify an exact client patch: the exporter
hardcodes that label and leaves version discovery as a TODO. Current external
buff descriptions can guide investigation but cannot close this pinned
evidence gap without explicit version reconciliation.

Mutation controls reject altered source identities, missing or crossed Barya
references, omitted stat meanings, unreviewed intrinsic fields or grant
factories, unrelated closure changes, and omission or false completion of
unresolved candidates.

The two old root test targets and their exclusive helpers are retired after
transferring the useful source, identity, semantic, census, structural-delta and
positive-chain laws to the maintained Authoring release target. The retained
archives and evidence remain inputs to ordinary offline proof tests; the
upstream license remains alongside the archived sources.
Current publication uses the [maintained KDL source workflow](../migration-source/README.md).
The [current source-domain proof](../structural-support-domains/README.md) adds
the six intrinsic support-source declarations independently. It does not extend
this historical packet's numerical or descendant claims. Run the maintained
checks with `cargo test -p poe-optimizer-authoring --test release structural_support`.
The six source tests retain the identity, semantic, structural and unresolved
census controls; four current-release tests retain the exact three-closure
inverse, both positive Barya chains and unchanged saved Original05 input status.
