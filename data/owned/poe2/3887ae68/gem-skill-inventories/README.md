# Four Gem potential-Skill inventories

This packet closes only `Gem.skills` on four existing definitions:

| Gem | Exact potential Skills |
| --- | --- |
| Frost Nexus `06c5` | Empty: its primary effect has the support role |
| Frost Bomb `0797` | `0149` |
| Ice Nova `07dd` | `01df` |
| Pain Offering `086b` | `02a2` |

IDs abbreviate existing `def.000000000000....` keys. No member, definition,
grant, Actor, numerical rule or runtime branch is added. The predecessor is
`520832751a5c6650954aa23c70f0b7a7b3ca5cc47c5cc39b8bd62277ee7f41e6`;
the migration uses the current schema-7, migration-5, operations-28 contract.

The full pinned Gem constructor establishes potential effect membership from
the primary and additional granted effects. The proof checks exact source
identities, winning declarations, support roles, all constructed effect
references and mappings to existing owned Skills. Activation, UI visibility
and current support compatibility cannot remove a potential member.
The source dispatch treats the Frost Nexus primary as a support effect.
Its creation of chilled ground remains an unresolved mechanic on supported
recipients; an empty potential Skill inventory does not make that effect inert.

Ice Nova declares two `additionalStatSet` aliases, `IceNovaPlayerOnFrostbolt`
and `IceNovaColdInfusedPlayer`. Its actual primary definition contains two
stat sets, Ice Nova and Cold-Infused. Both lists are retained independently.
Neither alias becomes an additional Skill, and alias count does not derive
the number of constructed stat sets. The existing two owned Action stat sets
`3283` and `3284` remain unchanged.

Seven complete source archives are reused by exact checked-in path, compressed
digest, expanded digest and byte limits. Nine reviewed boundaries cover source
loading, the full Gem constructor, saved identity resolution, support and active
Skill dispatch, and the separate downstream Actor factories. Source bodies are
authenticated against revision
`3887ae68a6a6b8bb7b41d1b61998f1aa184201e4` and the retained source manifest.
The complete identity catalog is also validated using its ordinary typed
loader. The independently sealed authoring document binds all evidence,
the predecessor descriptors and the shared upstream license.

The real Skeletal Sniper record remains a negative control: its declared and
constructed Command reference is unresolved even though the resulting effect
list contains only the summoner. Removing an unresolved reference from a
constructed list cannot establish completeness. Mutation controls also reject
wrong winners or roles, missing and additional effects, new constructor fields,
confusion between aliases and stat sets, changed positive members, and any
closure change outside the four approved fields.

Every Gem's direct grants, Actor declarations and skill supplies remain Partial,
including the existing positive primary grant chains. Every descendant Skill
and numerical gap also remains unchanged. Pain Offering explicitly creates a
Minion spike; Frost Bomb's Orb and Ice Nova's ground/infusion interactions still
need their own ownership review. Unmapped stat recipes are neither discarded
nor classified as harmless by this packet. This does not complete Scenario
intent discovery, legality, numerical evaluation or any unchanged build.

Ordinary Rust tests need no generated `runs/` package, vendor checkout, Lua or
network access. The whole-release publisher independently authenticates the
actual predecessor and checks the complete schema and recipe inverse.
