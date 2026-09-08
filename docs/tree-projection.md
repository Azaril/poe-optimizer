# Controlled passive-tree projection

`poe_optimizer_data::tree_projection` converts an authenticated
[tree snapshot](tree-data.md) into a finite core `CandidateCatalog`. The projection
keeps all **8 classes and 23 ascendancies**, their **28 shared physical implicit
roots**, and a caller-supplied set of allowed normal paid nodes. A candidate then
chooses its own subset of those paid nodes under explicit constraints.

This is a bounded preparation layer for class/ascendancy/passive mutations. It
neither calculates passive effects nor certifies complete game legality. The
initial `normal_paid_nodes_v1` policy intentionally rejects notable, keystone,
socket, image-only and otherwise unsupported source records. Expanding node
eligibility requires evidence about the affected allocation/provider mechanics;
it does not require dropping classes from the intended optimizer scope.

## Authenticate before projection

`AuthenticatedTreeSnapshot` has private immutable state and no `Deserialize`
implementation. It can be created through:

```rust
// Optional PoB adapter convenience wrapper:
poe_optimizer_pob::tree_projection::AuthenticatedTreeSnapshot::extract(
    executable, pob_source_root, timeout)
// Portable consumer with an independently trusted complete-content digest:
poe_optimizer_data::tree_projection::AuthenticatedTreeSnapshot::from_trusted_digest(
    snapshot, expected_sha256)
```

The first method calls the existing supervised extraction worker, including its
source verification, hard deadline and artifact bounds. The executable must be
the trusted application CLI. The adapter wrapper dereferences to the portable type,
preserving old `TreeProjection::new(&authenticated, ...)` call sites. The second
method checks the supported source identity and the **complete snapshot's
canonical content digest** against an
independently trusted expected digest. An expected digest copied from the same
untrusted input is not authentication. It must originate from a trusted local
extraction or separate trusted manifest. Pretty-printed file bytes have a
different digest from the canonical snapshot representation.

This distinction matters because `TreeDataSnapshot::validate_source_identity()`
only checks provenance claims. Changing a node's payload without changing the
identity fields still passes that identity-only check. The trusted full-content
digest constructor rejects such a changed snapshot.

Authenticated snapshots are immutable and internally shared using `Arc`; creating
several projections does not repeatedly copy the full source dataset. No Lua
handle or mutable VM state escapes extraction. Portable trusted producers can
use `from_trusted_extraction(snapshot, expected_sha256)` to record that route;
the producer must obtain the expected digest from its actual trusted extraction,
not labels or a digest supplied with untrusted input.

## Construct a finite catalog

```rust
let projection = TreeProjection::new(&authenticated, allowed_paid_node_ids)?;
let catalog = projection.catalog().clone();
let domain = CandidateDomain::new(catalog, explicit_constraints)?;
```

`allowed_paid_node_ids` is a choice catalog, not a requested allocation. Empty
sets are useful for identity-only fixtures. Unknown IDs, dangling targets and
implicit roots supplied as paid nodes fail construction. Every selected paid
record must be a normal source node, have ordinary or ascendancy accounting,
cost one source-default point and have no unsupported local metadata. Starts
must retain their implicit root type and source-default zero cost.

The current checks reject attribute/mastery choices, unlock constraints,
jewel/socket state, generated graphs, multiple-choice fields, and explicit
free/granted allocation metadata. Standard class/ascendancy switches are allowed
only when their source fields are covered: effective ID, name, stat lines, icon,
reminder text, ordinary display overlays and verified replacement ascendancy
labels. Topology, ownership or accounting changes outside that contract reject.
Unknown option names and unsupported effective overlay fields reject rather than
silently suppressing the affected class. If any selected node has an unsupported
option, the caller receives an error for that node; an unrelated supported choice
set still retains all eight classes.

Physical ownership remains intact. Witch/Sorceress share 54447; Ranger/Huntress
share 50459. Lich/Abyssal Lich share 23710, and supported paid node 58751 retains
both ascendancy owners. Class IDs are strings of the pinned integer IDs;
ascendancy IDs are the source internal IDs. The snapshot remains accessible
through `projection.snapshot()` for class names, attributes and effective source
records. These source views must later be compared with actual live calculation
observations, not treated as proof of the realized effects.

The graph is an induced subgraph of the authenticated usable adjacency: an edge
survives **only when both endpoints are projected**. Missing connector nodes are
never restored automatically, and no replacement path is invented. Valid source
edges crossing the projection boundary are recorded separately. Core catalog
validation checks root/owner references and the resulting graph once. Each
candidate still passes core connectivity, class/ascendancy compatibility, locks
and point-budget validation before evaluation.

Projection does not derive available points from character level or the number
of allocated nodes in an imported build. The caller supplies ordinary and
ascendancy point allowances in `CandidateConstraints`; roots cost zero, and paid
categories use independent budgets. Runtime-granted/free allocations, weapon
sets and derived budget changes remain outside this profile. An over-allocated
import must not become its own justification for a larger budget.

## Identity, composition and native consumption

The catalog identity binds projection schema/policy, the projection implementation
source hash, the complete snapshot digest and the ordered allowed paid IDs. Input
set insertion order cannot change it; a different choice set produces a different
identity. Class definitions, effective options and source graph changes are
already bound by the snapshot content digest.

The output `CandidateCatalog` initially contains no item, skill or support choices.
A higher-level producer can combine it with resolved equipment/skill catalogs,
but must **rebind the combined catalog identity** to every component and payload.
Reusing the tree-only identity after inserting items or skills would misidentify
the candidate domain. Keep source/projection evidence alongside that composition.

The projection and its owned Rust/serde model live in the Lua-free
`poe-optimizer-data` crate, with native and WASM support. Trusted local source
verification, Lua extraction and worker supervision remain in the optional PoB
adapter, which preserves its existing public paths through re-exports and a
convenience authentication wrapper. The production native backend borrows the
separately authenticated [bundled class/entrance subset](tree-data.md#bundled-subset-for-native-evaluation)
without linking PoB. That subset is a distinct type, never a trimmed object
presented as a complete snapshot. Projection itself does not calculate stats.

## Coverage and acceptance limits

`source_coverage()` retains the source identity/content digest, authentication
route, projection implementation hash, selected IDs, projected/unselected counts,
boundary edges, all 14 source dangling connections and the snapshot's global
unsupported categories. Unselected nodes include both otherwise supported choices
outside this finite catalog and unreviewed/special records; omission is not a
claim that those nodes are absent from the game.

Global source uncertainty is separate from selected-mechanic blockers. Copying all
snapshot categories directly into `CandidateCatalog.unsupported_mechanics` would
make every finite candidate unsearchable, including a no-paid-node identity
fixture. The snapshot's missing native stat translation can be covered by the
chosen backend for its explicitly supported profile; live-game completeness
still remains diagnostic. The projected catalog's passing finite rules therefore
coexist with explicit
coverage limitations. They must not be presented as unrestricted build legality.

Fresh backend checks must still compare actual class/ascendancy identities,
physical allocations, allocation modes, point counts, switched stat data, active
skills/resources and export round trips. Source-effective records alone do not
prove PoB applied an override, because live spec nodes retain physical IDs.
Current limits explicitly retain weapon sets, jewels, grants, alternate starts,
choice state and notable/keystone provider semantics as unmodeled.

Seven integration tests use an authenticated extraction from an isolated trusted
child. They cover every class/ascendancy identity, all 16 ordinary entrance cases,
shared roots/paid owners, separate point budgets, both-endpoint graph filtering,
a missing connector without repair, missing/special choices, unknown effective
overlays, tampered contents with unchanged source labels, stable identities and
coverage separation. The helper test is ignored in direct enumeration and invoked
by the parent tests. Targeted Clippy passes with warnings denied. These are
projection/finite-contract tests; live mutation parity remains a separate gate.

## Portable class/entrance composition

`poe_optimizer_data::class_tree` resolves a typed `ClassTreeSelection` against the selected
validated `GameDataSnapshot`. Its partial candidate catalog contains retained roots and
ordinary entrance nodes with exact owner sets and retained links. It preserves physical
allocation IDs and class-specific effective views, including Witch `4739 -> 17306` and
Huntress `56651 -> 39263`. Its identity binds the projection implementation and full dataset.
The controlled Mace adapter rebinds the final catalog to supplied skills/items and selected
tree choices. Caller budgets and locks are separate canonical constraints. This path
requires neither a complete extraction nor PoB at runtime; excluded nodes stay excluded.
