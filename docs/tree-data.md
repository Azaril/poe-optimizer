# Pinned passive-tree data snapshot

`poe_optimizer_pob::tree_data` extracts the pinned PoB `0_5` tree into owned,
serializable Rust data. The snapshot contains **4,914 physical nodes, 8 catalog
classes, 23 catalog ascendancies, 5,187 undirected usable edges and 14 dangling
connections**. These are properties of the committed PoB dataset, not a claim
about the latest live game. No website is scraped or consulted during extraction.

This is the data foundation for broader passive/class mutations and eventual
native calculation. It does not yet turn every extracted node into a certified
search choice. Existing [candidate validation](candidate-model.md) and
[tree-topology findings](tree-topology-investigation.md) remain separate concerns.

## API and ownership

The worker-side entry point is:

```rust
extract_pinned_tree(source_root: &Path, version: &str)
    -> Result<TreeDataSnapshot, TreeDataError>
```

Only `version == "0_5"` is supported. Other versions fail explicitly, without
falling back to another bundled tree. Run extraction in a supervised child
process so its deadline includes verification, parsing and serialization. The
synchronous library function itself cannot enforce a host wall-clock deadline.
It performs no file writes, full PoB startup, calculation, downloads or asset loads.

`TreeDataSnapshot` owns its maps, sets, strings and primitive values. Consumers
need no Lua state, handles, process IDs, filesystem paths or environment settings.
The types currently live in the PoB adapter crate; extracting a dedicated portable
data crate remains possible when native/WASM consumers need Rust linking. The
serialized schema can already be consumed independently of the Lua host.

Useful methods are:

- `ordinary_entrances(class_id)` returns the ordinary neighbors of the resolved
  physical class root, including reverse-listed source connections.
- `effective_node(class_id, ascendancy_internal_id, physical_node_id)` resolves
  the automatic class/ascendancy source option and its provenance. It rejects
  unknown identities and incompatible class/ascendancy pairs.
- `sha256()` hashes canonical compact JSON of the complete snapshot.
- `validate_source_identity()` rejects a stale schema, upstream revision, source
  manifest, tree/loader hash or extractor hash. This validates **identity claims**;
  it does not authenticate arbitrary deserialized payloads. A matching identity
  or a caller-supplied digest is not proof that snapshot contents are trustworthy.
  Consumers should use locally extracted artifacts or a separately trusted digest.

## Reproducibility and extraction limits

The identity records source revision
`3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`, tree version, snapshot schema version,
source-manifest SHA-256, individual hashes of `tree.lua`, `PassiveTree.lua` and
`PassiveSpec.lua`, and the extractor's own source hash. Source text normalizes
CRLF to LF, matching the existing source verifier. Ordered maps and sets remove
Lua table iteration order from serialized output. Two fresh extraction processes
produce identical JSON; ordinary JSON round trips preserve the same digest.

Extraction verifies the entire existing 1,082-file pinned source manifest first.
It then rereads the exact tree bytes and checks their normalized hash against the
embedded manifest before execution. Changing the tree between the initial check
and the read cannot substitute a different Lua program unnoticed.

A fresh `mlua` state loads no optional standard libraries and evaluates a text
chunk with an **empty environment**. This matters because mlua initializes Lua's
base library even with `StdLib::NONE`. The data chunk receives no `require`, file
loader, OS library, FFI, application modules or shared global state. Limits cover
8 MiB of normalized source, 128 MiB of Lua allocations, an instruction hook at
10,000-instruction intervals, 20,000 nodes, 200,000 raw connections, one million
copied values, 16 MiB of copied text and 32 levels of nested source tables. The
hook aborts after approximately ten million instructions. Parent process
supervision is still needed to bound source verification, Lua parsing, Rust-side
conversion and artifact serialization. This is a bounded extraction of trusted,
hash-verified local data, not a general-purpose untrusted Lua sandbox.

Every selected source record is retained as a `SourceTable`: separate ordered
`named` and `indexed` maps preserve string versus numeric Lua keys. Tagged
`SourceValue` variants retain booleans, integers, finite numbers, strings and
nested tables. Empty tables stay tables, and numeric attribute-choice indexes
are not guessed to be JSON object field names or arrays. Functions, metatables,
unsupported key types, non-finite numbers and excessive nesting fail explicitly.
This preserves source fields that do not yet have native semantic implementations.

## Physical identities and effective source data

Class selection uses the exported `classes` catalog's `integerId`, rather than
legacy names in start labels or array positions. Witch/Sorceress share root
**54447**; Ranger/Huntress share **50459**. All eight classes retain two ordinary
entrances. Shadow, Marauder, Duelist and Templar start labels do not create
selectable classes at this source revision. Class records retain base attributes,
ascendancy membership and complete source metadata.

Ascendancies use their internal IDs and retain the owning class, one-based class
index, catalog name, replacement metadata and physical start. Lich (`Witch3`) and
Abyssal Lich (`Witch3b`) share root **23710**. Their switched paid nodes share
physical IDs too: **58751** belongs to both source alternatives. Ownership sets
preserve these relationships without inventing additional graph nodes.

Automatic switches follow the class-name lookup first, then ascendancy-name
lookup in `PassiveSpec:BuildAllDependsAndPaths`. The chosen option shallowly
inherits missing source fields from the base, matching the source-table fallback
established by `PassiveTree:ProcessNode`. The effective view records its base
physical ID, effective source ID, selected option, overridden field names, name,
stat lines and inherited source payload. For example:

| Physical node | Selection | Effective source ID | Evidence |
| --- | --- | --- | --- |
| 4739 | Sorceress | 4739 | Base Spell Damage |
| 4739 | Witch | 17306 | Spell and Minion Damage class option |
| 56651 | Ranger | 56651 | Base Projectile Damage |
| 56651 | Huntress | 39263 | Attack Damage class option |
| 58751 | Lich | 58751 | Base ascendancy record |
| 58751 | Abyssal Lich | 35941 | Ascendancy option at the same physical node |

The Abyssal Lich **start** option has no explicit ID, name or stats. Its effective
source view therefore inherits ID 23710, display name `Lich` and empty stats while
retaining the overridden `ascendancyName` as `Abyssal Lich`. Do not infer a fresh
physical root or silently replace the inherited display name.

This method describes **effective source data**. It does not claim every source
field replaces the corresponding field on PoB's live `PassiveSpec` node:
`ReplaceNode` copies selected display/modifier fields while retaining physical
identity and connectivity. Attribute choices, explicit hash overrides, jewels
and runtime-granted transformations require additional provenance and are not
resolved by this automatic-switch method.

## Graph, point categories and unsupported mechanics

Nodes are keyed by `skill`, matching the loader's physical node map. Extraction
retains the original directed connection targets and separately constructs
bidirectional usable adjacency. It skips missing targets, image-only endpoints
and self-connections in the loader's order, retaining each reason as evidence.
Cross-ascendancy and class-start connectors remain in the observed graph; PoB's
later decision not to draw some connectors does not remove their allocation links.
The snapshot itself is therefore not a general path validator.

Implicit class/ascendancy roots have source-default cost zero; ordinary and
ascendancy paid nodes have source-default cost one in their separate categories.
Image-only records are nonallocatable. Multiple-choice options have unsupported
accounting instead of an invented ordinary cost. Runtime free/granted allocation,
weapon-set use, progression rewards and derived point grants still require
separate checks; a cost field is not authorization to allocate the node.

Per-node diagnostics identify attribute choices, sockets, embedded sockets,
generated subgraphs, unlock constraints, mastery and multiple-choice records.
Global unsupported categories explicitly retain stat/modifier translation,
weapon-set allocation, alternate/radius starts, free/granted allocation,
jewels/subgraphs, explicit hash overrides, unlock constraints, choice semantics,
progression/derived point budgets and completeness relative to the live game.
Full source records retain metadata for later implementations. The 14 absent
targets remain evidence of unknown coverage; extraction neither synthesizes
replacement nodes nor declares that they are irrelevant.

## Validation evidence

Seven integration tests consume child-process extractions of the actual pin.
They cover all eight class roots and both entrances, reverse-listed edges,
every usable edge's symmetry, the exact 14 dangling pairs, shared class and
ascendancy ownership, shared paid nodes, effective override IDs and inheritance,
class-before-ascendancy precedence, invalid identity pairs, separate point
categories and unsupported flags. They also verify stable JSON and digests
across fresh processes and serialization round trips, reject stale provenance,
and reject a modified local tree before its Lua can execute. An ignored helper
test is invoked only as the isolated extraction child. Targeted Clippy passes
with warnings denied.

These tests validate extraction and the reviewed loader behavior, not independent
numeric parity for every passive's effects or full gameplay legality. The next
useful checks are minimal evaluated builds for every class/ascendancy and both
ordinary entrances, followed by requested-versus-realized passive identity and
override comparisons before unrestricted tree mutation enters search.
