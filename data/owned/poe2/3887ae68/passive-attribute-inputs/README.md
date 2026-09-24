# Complete plain passive attribute inputs

This policy converts **58 complete passive stat lists** from the authenticated finite
tree catalog through the existing `compile-owned-passive-views` command. It covers 51
flat-attribute nodes and seven increased-attribute nodes: 60 source lines produce 94
contributions. Selection is by reviewed complete source lists across the full catalog,
not by a build fixture, display name, or runtime source-text parser.

Flat values use the existing Player Integer channels `1d2e` Strength, `1d2f` Dexterity
and `1d30` Intelligence. Increased values use `Contribute(Increase)` with the existing
PercentagePoints unit `0002` on those same channels. Keys have prefix
`def.000000000000` in `poe2/owned-mechanics-v1`. All Attributes emits three effects
from one physical node; paired source lines retain their individual contributions.
No new definitions, opcodes, tables, receivers or runtime source dependencies are added.

Each policy row preserves the exact full ordered stat list and ordinary or ascendancy
point pool. All selected nodes have no alternate views or unlock effects. Optional source tests
also review raw metadata and reject socket, choice, special equipment and deferred control
semantics; matching displayed stat text alone is not proof of full node semantics. The existing
converter checks the complete list and membership before closing only the node's reviewed
declaration ports and numerical program list. It does not grant allocation access, alter
point costs, change topology, or declare the overall build complete. An ascendancy pool
remains distinct from the ordinary pool.

Mixed-effect lists are deliberately excluded. Examples include Pure Energy's Energy
Shield effect, Insightfulness's Energy Shield and mana regeneration effects, and Enhanced
Reflexes's evasion and deflection effects. Their attribute lines alone cannot justify
complete node coverage. Alternate views, unlocks, conditional effects, MORE products,
requirements and inherent-attribute bonuses also remain separate work.

The native publication uses an explicit predecessor and an empty statistics input:

```text
poe-optimizer compile-owned-passive-views PRIOR --catalog data/owned/poe2/3887ae68/tree/tree-catalog.json --policy data/owned/poe2/3887ae68/passive-attribute-inputs/policy.json --statistics EMPTY_ARRAY_JSON --output NEW
```

`EMPTY_ARRAY_JSON` is a caller-owned UTF-8 file containing `[]`. The predecessor for this
checkpoint is the actor-attribute successor; no machine-local path is built into the CLI.
The host preserves its checked predecessor's compact or legacy publication format. Compact
publication avoids a duplicated recipe and uses the existing separately bounded commitments;
all schema-refinement and preservation checks still apply.
`bindings.json` records the exact catalog/source provenance and owned node identities for
review. The checked publisher, rather than that manifest, establishes package authority.

The six selected-node witnesses in the five originals are original01's `50816`,
original03's `20437` and `50755`, and original04's `364`, `34202` and `49285`.
These are validation witnesses, not production selection rules. Original02 and original05
gain no selected nodes in this bounded conversion. Their saved build data, all 110 queries,
140 Shared skill scopes, 12 Complete/466 Pending gem parameter lists, 64 admitted modifier
occurrences and 53 observed displays remain unchanged. Allocation access remains pending.

This adds real providers to the shared numerical path. Final attributes still need staged
receivers, complete contributors and downstream resource calculations. No original-build
parity or final attribute value is established by these provider programs.
