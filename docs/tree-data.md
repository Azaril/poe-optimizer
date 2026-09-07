# Pinned passive-tree data snapshot

`poe_optimizer_data::tree_data` owns the portable snapshot model. The optional
`poe_optimizer_pob::tree_data` adapter extracts the pinned PoB `0_5` tree into owned,
serializable Rust data. The snapshot contains **4,914 physical nodes, 8 catalog
classes, 23 catalog ascendancies, 5,187 undirected usable edges and 14 dangling
connections**. These are properties of the committed PoB dataset, not a claim
about the latest live game. No website is scraped or consulted during extraction.

This is the data foundation for broader passive/class mutations and
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
The models, source-identity checks, canonical hashing and finite projection live
in the Lua-free `poe-optimizer-data` crate, which compiles for native and WASM
targets. The PoB adapter re-exports the old public model paths for compatibility.
Only local source verification, Lua extraction and worker supervision remain in
the optional reference adapter.

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
`PassiveSpec.lua`, and a combined hash of the extractor and portable model source.
Source text normalizes CRLF to LF, matching the existing source verifier. Ordered maps and sets remove
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
legacy names in start labels or array positions. Snapshot schema **2** also
preserves the one-based source array position as `TreeClass.source_index`;
Warrior has source index **3** and canonical integer ID **6**. These values are
not interchangeable. Schema-1 snapshots are rejected; regenerate them from the
pinned source rather than filling the new field from an assumed index. The
bundled subset has its own schema version **1** and is a different data type.
Witch/Sorceress share root **54447**; Ranger/Huntress share **50459**. All eight classes retain two ordinary
entrances. Shadow, Marauder, Duelist and Templar start labels do not create
selectable classes at this source revision. Class records retain base attributes,
ascendancy membership and complete source metadata.

Ascendancies use their internal IDs and retain the owning class, their one-based
selection index within that class, catalog name, replacement metadata and physical
start. Lich (`Witch3`) and Abyssal Lich (`Witch3b`) share root **23710**. Their switched paid nodes share
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

## Bundled subset for native evaluation

`poe_optimizer_data::bundled::class_tree()` borrows an immutable, once-parsed
`BundledClassTree`; it needs no checkout, Lua runtime, process or filesystem
access. The separately typed subset contains all **8 class identities and 23
ascendancies**, **28 implicit physical roots** and **12 ordinary physical nodes**
with **16 class-specific entrance views**. It retains the full source identity,
full snapshot digest, exact raw records, class/ascendancy switches, effective
stat strings and override provenance. See the crate's
[NOTICE](../crates/poe-optimizer-data/NOTICE.md) for source and game-data attribution.

The checked-in `class-tree.json` is authenticated against a separately compiled
`class-tree.sha256` before use. `authenticate_bundle(bytes)` accepts only those
exact reviewed bytes; a caller cannot supply its own expected digest. The
artifact digest and the portable model/projection implementation fingerprint
are bound into native backend identity. A matching provenance label alone never
admits replacement data.

The subset explicitly records **4,874 excluded nodes**, all **14 source dangling
connections**, retained-to-excluded boundary edges and coverage limitations.
Raw retained adjacency still references excluded endpoints; it must not be used
as a complete allocation graph. Only the `class_entrances` map is an admitted
native entrance choice set. Borrowed `class`, `ascendancy`, `entrances` and
`entrance` accessors resolve canonical IDs and reject incompatible owners.

Generation checks every class-only and class/ascendancy selection: all **31**
choices have stat-free implicit roots without unhandled root mechanics, and each
entrance's effective source view is unchanged by every owning ascendancy. Unknown
root fields, root stat effects, changed entrance overlays or point semantics fail
subset validation. Shared and inherited root display names remain source evidence;
they do not add modifiers. Allocated ascendancy effects are outside this subset.

This artifact supplies exact data, not a generic stat translator. The current
[native backend](native-backend.md) maps the reviewed entrance stat strings and
admits zero or one ordinary entrance connected to the selected class root. Point
allowances remain explicit caller constraints. `validate_scope()` checks subset
semantics but does not authenticate arbitrary deserialized data.

The adapter's `tree_bundle` integration test creates a fresh full snapshot in a
supervised child, derives this subset, and compares its canonical bytes with the
compiled artifact. It proves all retained data and provenance reproduce from the
pinned source. Six Lua-free data tests separately check authentication, identity
mapping, shared roots and class-specific effects, owner rejection, and rejection
of unmodeled root mechanics. Data tests and WASM compilation require no PoB source.

## Planned general data boundary

This compiled tree subset and its global loader are the current implementation. The
[game-data decision](game-data-boundary.md) extends the portable model to injectable
versioned packages for all supported game content. The default bundle will use the same
validated loading path as external bytes. Existing exact source checks remain in force
until an explicit data/semantic compatibility contract replaces them; this design does not
make arbitrary edited bundles valid for the current loader.

## Reproduce or deliberately refresh the bundle

Run from the repository root with the pinned reference submodule available:

```powershell
cargo test --locked -p poe-optimizer-pob --test tree_bundle compiled_bundle_matches_fresh_full_source_extraction
cargo test --locked -p poe-optimizer-data
cargo check --locked -p poe-optimizer-data --lib --target wasm32-unknown-unknown
```

The first command supervises a private child with a 60-second deadline and bounds
its artifact before reading it. It does not rewrite the committed bundle.

For an intentional refresh, first review the source/model changes and update
`crates/poe-optimizer-data/data/tree-source-identity.json`. Keep its pin, full
manifest hash and three source-file hashes aligned with the adapter's reviewed
source manifest. Its `extractor_sha256` is SHA-256 of the concatenation, in order,
of the UTF-8 bytes of `poe-tree-extractor-and-model-v2`, the entire
`crates/poe-optimizer-pob/src/tree_data.rs`, and the entire
`crates/poe-optimizer-data/src/tree_data.rs`; normalize CRLF to LF in both files.
There are no separators or length fields. Format the Rust files before hashing.
A source/model change with an unchanged expected hash fails extraction before
Lua execution. Other portable implementation changes still change the data and
backend implementation fingerprints. Native preparation also requires the bundle's
rules revision, tree version and tree hash to match both numerical pipelines. Update
those numerical source declarations only with the corresponding source/parity review;
the compatibility guard rejects mixed data/formula pins before calculation.

After reviewing the identity, the explicit offline generation helper writes a
fresh candidate artifact to a unique local path:

```powershell
$taskOutput = Join-Path (Get-Location) ("runs/tree-bundle-" + [guid]::NewGuid() + ".json")
$env:POE_TREE_BUNDLE_OUTPUT = $taskOutput
cargo test --locked -p poe-optimizer-pob --test tree_bundle -- --ignored --exact bundle_extraction_child
if ($LASTEXITCODE -ne 0) { throw "Bundle generation failed" }
Get-FileHash -LiteralPath $taskOutput -Algorithm SHA256
```

This helper is for trusted development generation; production extraction uses the
supervised worker. Inspect the generated records, coverage and provenance diff,
then deliberately replace `data/class-tree.json` with those exact bytes and
`data/class-tree.sha256` with their lowercase SHA-256 followed by a newline. Do
not derive a new trusted digest from an arbitrary supplied bundle. Repeat the
supervised reproduction and portable checks above, plus live parity for affected
native profiles. Changes to scope, mechanics or source contracts require an
explicit schema/policy review; updating a checksum cannot authorize new effects.

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
numeric parity for every passive's effects or full gameplay legality. Native
contract and optional live-reference parity tests cover the admitted class and
entrance profiles separately. Unrestricted tree mutation still needs broader
allocation and modifier evidence.
