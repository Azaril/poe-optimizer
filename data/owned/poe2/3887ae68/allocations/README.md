# Owned allocation cost and budget component

`rules.json` is a source-independent `AllocationRulesInput` bound to the current owned schema. It contains 4,503 exact node/pool costs: 4,500 one-point rows and three explicit zero-point rows. The 28 implicit roots, 15 attached option tokens and 368 presentation rows are excluded. IDs come from the existing positive tree mappings; this export neither allocates IDs nor modifies the current bundle.

The three declared budgets are ordinary shared-plus-maximum-scoped usage, ordinary each-scope usage excluding shared nodes, and total ascendancy usage. Every acquired capacity is explicitly Unmapped. The budget registry is Partial because additional point-budget constraints are not converted. That closure describes budget membership; allocation access is a separate concern and is not supplied here. No endgame cap, quest completion, trial completion, missing-to-zero rule or legality certificate is emitted.

The native loader consumes only `rules.json` and its exact owned schema. `source-policy.json` and `source-facts.json` are retained offline provenance; neither belongs in the native evaluation, GUI or search input. Rust source-proof tests read finite JSON and pinned source bytes without evaluating Lua, importing builds, replaying UI state, or constructing a source interpreter. Native constructors validate the stored DTO before use.

The reviewed cost conversion follows `PassiveSpec.lua` CountAllocNodes: roots/options are excluded by semantic role, and `isFreeAllocate` being present means zero paid cost. The source counter uses a nil test; even an explicitly false field is non-nil. This source encoding is translated here into an explicit integer and does not enter the owned runtime. Sanguimancy, Smith's Masterwork and Sacred Unity are the three current zero-cost physical definitions.

Run the retained source-proof tests from the repository root:

```powershell
cargo test -p poe-optimizer-import --test owned owned_allocation_source::
cargo test -p poe-optimizer-data --test owned owned_allocations::
python scripts/tests/test_export_owned_tree.py
```

The generic Python allocation emitter and its publication-only tests have been retired; their implementation remains in Git history. There is no replacement generic Rust allocation export command yet. Future source updates need a reviewed Rust acquisition path before regenerating this packet. The existing strict Python tree exporter remains active and independently verifies the complete raw-source field inventory, classified catalog, duplicate-key rejection, repinned unknown-field rejection and source/output bounds.

`crates/poe-optimizer-import/tests/owned_allocation_source.rs` authenticates the unchanged packet and all six pinned source files, verifies both current bundle manifest layers, and consumes the standalone schema through the sole Data compact codec. It requires exact semantic equality with the checked compiler recipe and exact registry/schema/mapping/tree/base-policy bindings. Its full census joins source nodes, classified roles, exact mappings, point-pool membership and every stored cost; it retains the three budget usage policies and precise unknown-capacity/Partial gaps. Detached mutations cover missing/duplicate/changed costs, wrong pools, free-field removal, missing raw nodes, source corruption, duplicate roles and foreign identities. These ordinary tests require the pinned submodule checkout, but no Lua runtime.

`source-facts.json` is preserved byte-for-byte as historical evidence. Its `bundle_transition_sha256` records the original expanded-schema transport and is not the hash of today's compact bundle. Its semantic bindings and `rules.json` hash remain current and are checked independently. Generic native allocation, tree-policy, mapping and artifact-loader tests retain typed rejection and budget laws; Host publication tests retain atomic no-clobber behavior. The retired emitter's exact output formatting and generic publication interface are no longer supported. Source fingerprints record reviewed compatibility, not proof of an original exporter's identity.

The five originals, fixed 110 numerical expectations, semantic bundle content and identities, existing registry history and optional source oracle remain unchanged. Compact schema storage changes the bundle's transport artifacts and receipts. This component does not establish original-build parity.
