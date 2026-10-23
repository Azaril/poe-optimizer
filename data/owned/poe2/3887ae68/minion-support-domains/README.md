# Reviewed numeric Minion passive support domains

This source packet supports intrinsic support-origin absence for exactly 18
reviewed passive nodes at Path of Building revision
`3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`.

| Retained family | Exact source node IDs |
| --- | --- |
| Plain Minion damage | 8737, 95 |
| Minion damage and life | 19006, 229, 39461, 54453 |
| Minion life | 1218, 40894 |
| Command cooldown, including paired damage | 14598, 35645, 4345, 43979, 50837, 6077 |
| Command damage | 25927, 32847, 41511 |
| Growing Swarm | 14945 |

The inventory is explicit. It is not expanded to other nodes with matching
display text. The 27 complete source stat rows correspond to 26 existing
passive programs because Growing Swarm has two effects in one program. These
counts supplement exact identity, body and effect checks.

`evidence.json` reuses the existing full archives for executed
`src/TreeData/0_5/tree.lua`, the sibling raw `tree.json`, `PassiveTree.lua`,
`PassiveSpec.lua` and `ModParser.lua`. The fixed source manifest authenticates
the Lua archives. The raw JSON has its own fixed digest and is connected to the
executed data by complete literal equality. No duplicate archives, source
checkout, Lua runtime or generated game parser are required.

The test in
`crates/poe-optimizer-authoring/tests/release/minion_support_source.rs` checks
every field of each raw node. Seventeen records have the exact ordinary
nine-field shape. Growing Swarm additionally has `isNotable=true` and the exact
three-item recipe. The test rejects added options or other raw behavior,
changed stat rows or conditions, null notable/recipe fields and unsafe quoted
text. A fixed-shape encoder compares the entire raw JSON record with the
authenticated Lua node literal, including connections, metadata and closing
table boundary. Node ranges are scoped to the root `nodes` table; other tables
also use numeric keys. All 18 catalog rows and retained source-vector bodies
must agree with those complete records.
The catalog must also contain no attached choice owned by these nodes and no
edge from any of them to a class root, preserving the independent checks against
additional choice inputs and automatic class-start condition injection. Detached
catalog mutations exercise both refusals.

The source review follows the entire tree constructor, `ProcessStats` and
`ProcessNode`. Ordinary and notable classification is explicit. Recipe
processing supplies presentation information, and the switchable/attribute
option path cannot apply to these exact raw records. The constructor may add a
class-start connection condition flag; that path is included. `ProcessStats`
parses every row, preserves unknown/partial parsing status and installs only
recognized complete modifiers. Failed parsing is not treated as a source-domain
proof.

The numeric rows use the parser's `INC` form. Literal modifier names are
`Life`, `Damage`, `AreaOfEffect` and `CooldownRecovery`. The Minion prefixes
select `addToMinion`, and the existing final wrapper emits a `MinionModifier`
containing the scalar modifier. The phrases “with Command Skills” and “for
Command Skills” add the existing `CommandableSkill` condition through
`modFlagList`; the condition survives the ordinary tag merge. Growing Swarm's
two rows are unconditional area and cooldown increases. The full parser file,
complete parsing function, scanner, form table, cache wrapper and exact named
dispatch entries are authenticated. Earlier special-modifier and jewel-function
branches are included in the full source review; none of their origin-creating
patterns matches these finite complete rows.

Positive contrasts retain actual `ExtraSupport`, `LinkedSupport` and
`ExtraMinionSkill` factories. `PassiveSpec` override application and external
jewel replacement remain explicit boundaries. The claim does not certify those
external suppliers, remove inherited candidates, promote other owner domains,
or establish complete numerical coverage. A Command condition being false or
unresolved is separate from whether the passive creates an intrinsic support
origin.

The packet fixes hashes for the retained catalog and exact mapping, six binding
files and six source-vector files, four closure payloads, and the Command cooldown and
Growing Swarm migrations. The historical closures and migrations are immutable
semantic references for the maintained runtime tests. Their old package seals
are not repaired or admitted as current artifacts, and report paths inside
source-vector files are never opened. The sibling `minion_support_domains.rs`
derives current typed owners through these authenticated inputs and checks
the existing DSL and evaluator laws independently of the new support-domain
inventory.

The retained observations also keep their complete nested modifier records and
multiplicity: paired damage/life, paired damage/conditional cooldown, and both
Growing Swarm modifiers. They describe the default tree definition's modifier
list. `effective_same_definition=false` compares a mutable spec wrapper's object
identity with the definition; it does not prove effective transformations absent.
The original non-observation of Growing Swarm area receiving is retained.

Rejection controls start by requiring the unmodified proof to succeed, then
exercise missing or retargeted node evidence, relocated spans, changed binding
commitments, removed parser evidence, added raw behavior and condition changes.
The production change remains limited to the reviewed source-domain declarations.
