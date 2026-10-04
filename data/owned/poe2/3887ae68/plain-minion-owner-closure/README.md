# Two complete default Minion Damage passive owners

This packet closes the existing default definitions for source nodes 8737 and 95
(owned Passive `1b84` and `1bc9`). Both are selected in Original05. Each complete
source node has one unconditional `Minions deal 10% increased Damage` line, and
the existing `ordinary-minion-damage` program already contributes that value to
Player channel `1d33`. The packet changes no program body or number and allocates
no definition IDs.

`closure.json` contains two replacement descriptors and two replacement owner
rows. Only the seven empty declaration closures on each node and its program
inventory change to Complete. `dependencies.json` retains the exact predecessor
rows. Pools, adjacency, schema members, receiver applicability, all other owners,
tables, routes and query templates remain unchanged. In particular, the eight
other selected nodes in the previously authored Minion Damage family have a
second source effect and retain their Partial owners.

The scope is the owned default passive definition. The source catalog has no
alternate views, unlocks or attached choices for either node; exact source
mapping uses `view: Missing`. Full pinned Lua node bodies contain no mastery,
socket, grant, actor, action or special-effect metadata. Neither node is adjacent
to a class start, so PassiveTree's automatic `ConnectedTo*Start` flag does not add
another modifier. External effects that transform a passive belong to their
own provider and keep their independent coverage obligations. This packet does
not declare those providers or all Minion recipients complete.

`source-vectors.json` retains bounded exact observations from the existing full
physical-damage witness, plus complete static node text and catalog/mapping
correspondence. Both JIT reports contain 37 cases and 38 complete load attempts,
are 19,612,949 bytes, and share SHA256
`030f14d71a01a2c54862eb858249b2abca1ba1caa9337dbffb3458118777e935`.
The selected original contains one untagged `MinionModifier` LIST record with
inner `Damage/INC/10` for each node. The retained observations also include all
five originals, repeated/warm Original05, and the control that removes node 95
and consequently prunes 8737. Static source and original runtime observations
are separate evidence; the runtime's copied effective node is not assumed to be
the static table object.

The test publication helper uses the existing checked compact successor and
`PassiveDeclarationRefinement` APIs. It authenticates predecessor bindings,
source pins and both report byte commitments before publication. It restores
only the reviewed closures and exact dependency identities, then compares the
entire remaining recipe and release. No new migration version, compiler or
runtime exception is introduced. Ordinary CI checks the tracked packet and
manifest without loading PoB or ignored reports.

Both publication tests pass. The checked package is
`runs/owned-plain-minion-owner-closure-01/package`, input
`75d4189c657a3a39e4c80fe3d6358aeb7a6e2a42b1caba6a22f3643f216c0857`.
Its eighteen files total 60,787,062 bytes with 83 provenance rows and rebuild
byte-for-byte. All five original input projects, sidecars, saved selections,
local IDs and 110 queries are preserved. Selected unresolved input counts remain
113/116/108/121/11. Closing two selected owners does not resolve the remaining
input, topology, contributor, support, encounter or numerical coverage gaps.
No evaluation bundle or whole-build parity is claimed.

Three new native closure tests and the six existing passive tests pass. They
reuse the explicitly finite passive topology and final-level fixture: the exact
published owners contribute 10+10; prior or other selected Partial owners still
block totals; unselected Partial catalog rows do not; scratch reuse remains
stable. The tests do not assert full-tree legality or complete original builds.

For an opt-in publication check, use a new, nonexistent output directory:

```powershell
$env:POE_OPTIMIZER_TEST_MINION_OWNER_CLOSURE_PRIOR = 'runs/owned-amulet-level-copy-02/package'
$env:POE_OPTIMIZER_TEST_MINION_OWNER_CLOSURE_OUTPUT = 'runs/owned-plain-minion-owner-closure-01'
cargo test --test owned_plain_minion_owner_closure_cli -- --include-ignored
```

The parent `runs/owned-minion-physical-damage-source-02` reports and pinned source
checkout are required only for that opt-in publication proof. The existing source
witness need not be rerun when those exact evidence bytes remain available.
