# Constructed item layout evidence

This finite import catalog records whether each constructed source base prepends
flask/charm buff lines before item modifiers. It is an import-layout contract;
it adds no PoB-shaped field to the owned build or native evaluator.

`catalog.json` is exported by the optional Rust PoB adapter from the final
constructed base catalog. `evidence.json` records verified source hashes and the
exporter identity. The original item-base v1 catalog and its bytes remain unchanged.
The pinned source produces 1,756 bases: 1,743 have no generated prefix, 13 have a
prefix, and none are unsupported. An empty string inside a buff sequence still
creates a source line; Cleansing Charm therefore has a prefix.

The exporter inspects actual `flask.buff` and `charm.buff` membership. It does not
infer absence from item type, weapon presence, an unselected branch or a failed
lookup. Absent fields and reviewed empty sequences establish absence; reviewed
nonempty string sequences establish presence. Unsupported shapes remain unknown.
The optional adapter authenticates construction; copied source hashes in an
arbitrary supplied JSON catalog are compatibility bindings, not authentication.

`policy.json` binds the exact catalog bytes and the predecessor's schema, item
line policy and source-layout policy. Every source base maps to its existing
owned template and exact literal header rule. Apply after the 10,537-entry
elemental weapon successor:

```powershell
poe-optimizer compile-owned-item-layouts PRIOR --catalog data/owned/poe2/3887ae68/item-layouts/catalog.json --policy data/owned/poe2/3887ae68/item-layouts/policy.json --output NEW
```

The native compiler refines only unresolved generated-prefix declarations backed
by explicit absence evidence. Presence and unsupported evidence stay unresolved;
contradicting a previous absence declaration is rejected. Whole-catalog bijection,
source pins, artifact identities, existing header semantics and bounded work are
validated before publication. Every unrelated source field, item policy, schema,
rule program, tree policy and query remains intact. Replay uses the same predecessor;
a different predecessor needs an explicitly rebound policy.

Knowing a base has no generated prefix does not prove a whole item. Unknown
headers, implicit/rune lines, source modifier indices, eligibility, ordered effect
membership and collection completeness still need their own evidence. This proof
can unblock range attribution and omitted-input defaults only when the remaining
item layout is admitted. No complete-build parity is claimed.

Regenerate acquisition evidence separately with the optional adapter:

```powershell
cargo run -p poe-optimizer-cli --features pob -- export-owned-item-layouts --source-root vendor/path-of-building-poe2 --output NEW_EXPORT
```

All new acquisition, conversion and regression code is Rust. The native command
consumes finite data without a PoB checkout or Lua runtime. Export remains outside
repeated evaluation and search.
