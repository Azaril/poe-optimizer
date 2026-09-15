# Owned allocation cost and budget component

`rules.json` is a source-independent `AllocationRulesInput` bound to the current owned schema. It contains 4,503 exact node/pool costs: 4,500 one-point rows and three explicit zero-point rows. The 28 implicit roots, 15 attached option tokens and 368 presentation rows are excluded. IDs come from the existing positive tree mappings; this export neither allocates IDs nor modifies the current bundle.

The three declared budgets are ordinary shared-plus-maximum-scoped usage, ordinary each-scope usage excluding shared nodes, and total ascendancy usage. Every acquired capacity is explicitly Unmapped. The budget registry is Partial because additional point-budget constraints are not converted. That closure describes budget membership; allocation access is a separate concern and is not supplied here. No endgame cap, quest completion, trial completion, missing-to-zero rule or legality certificate is emitted.

The native loader consumes only `rules.json` and its exact owned schema. `source-policy.json` and `source-facts.json` are offline tooling/provenance; neither belongs in the native evaluation, GUI or search input. The exporter reads finite JSON plus pinned source bytes, never evaluates Lua, imports builds, replays UI state, or emits a source AST/interpreter. Native constructors still validate the emitted DTO before use.

The reviewed cost conversion follows `PassiveSpec.lua` CountAllocNodes: roots/options are excluded by semantic role, and `isFreeAllocate` being present means zero paid cost. The source counter uses a nil test; even an explicitly false field is non-nil. This source encoding is translated here into an explicit integer and does not enter the owned runtime. Sanguimancy, Smith's Masterwork and Sacred Unity are the three current zero-cost physical definitions.

Reproduce from the repository root:

```powershell
python scripts/export-owned-allocations.py --manifest data/owned/poe2/3887ae68/tree/source-manifest.json --policy data/owned/poe2/3887ae68/allocations/source-policy.json --catalog data/owned/poe2/3887ae68/tree/tree-catalog.json --source-root vendor/path-of-building-poe2 --bundle data/owned/poe2/3887ae68/current --check-dir data/owned/poe2/3887ae68/allocations
python -m unittest discover -s scripts/tests -p test_export_owned_allocations.py -v
```

For a new export, replace `--check-dir` with `--output-dir` naming a nonexistent directory. All validation and bounded serialization finish before that directory is created; existing output is never overwritten. The CLI writes `rules.json` and `source-facts.json`; the reviewed source policy remains an explicit input.

Input verification covers both bundle manifest layers and all declared file byte hashes, then exact registry/schema/mapping/tree/base-policy bindings for the consumed graph. The existing strict tree exporter must reproduce the supplied classified catalog; unknown source fields, pin conflicts, missing/ambiguous mappings, mismatched roles, duplicate identities and incomplete point-pool membership reject. This is not a substitute for the generic native schema/rules constructors or successor publication validator. Source fingerprints record reviewed compatibility, not proof of an original exporter's identity.

The five originals, fixed 110 numerical expectations, current bundle, existing registry history and optional source oracle remain unchanged. This component does not establish original-build parity.
