# Current owned package

This is the single published predecessor for new owned catalog work: registry watermark **7469**, with the complete earlier 2589-entry prefix retained. Use its `recipe.json` and `mapping.json` when adding definitions. Older import/resistance packages are reproducible historical stages, not alternative allocation bases. Changes to existing descriptors require an explicit refinement policy.

The offline tree extension adds 4582 definitions and 298 choice slots. It represents eight classes, 23 ascendancy choices, 28 physical roots, ordinary/ascendancy pool eligibility, physical adjacency and exact parent choices. Roots belong to the character; attached options are choices, not paid allocations. Fourteen links to missing source nodes remain unresolved adjacency. Known structure does not close untranslated effects, class-dependent views, unlock conditions, radius access or legality budgets.

The **20 generated JSON files** are emitted by the existing `publish-owned-successor` followed by `extend-owned-tree-catalog`. Both use the same checked finalizer and publisher. The tree command validates the complete prior manifest, preserves old identities/declarations/programs, appends only new selectors, validates prior import policies before rebinding, and installs the tree policy in one final manifest. Repeating the same catalog allocates no IDs and preserves semantic artifacts exactly. A changed tree policy requires an explicit migration. Neither Rust command requires a PoB checkout or executes Lua.

`schema.json`, `rules.json` and `routing.json` are portable native semantic inputs. `registry.json` is the authoring identity ledger. Source mappings, roles, normalization/reward/item policies, `tree-normalization.json`, query lists and `catalog-append.json` belong to offline/import tooling. The native evaluator never interprets that source syntax. `recipe.json` assembles the owned packages for offline validation. Manifests record identities and hashes; they do not authenticate provenance or certify numerical completeness. These expanded JSON authoring artifacts are not a compact runtime distribution or a performance claim.

All five originals normalize with **1333 physical allocations, 335 allocation choices and 32 implicit-root origin links** across all 16 saved tree specifications. Their 116 items, 478 gems, 81 reward selections, 195 equipment uses and 110 ordered query rows remain. All allocation access remains pending; point budgets and numerical effects are not yet established. **0/5 whole original builds complete natively.** Use `normalize-owned --tree-policy <this directory>/tree-normalization.json` alongside the other emitted policies to enable structural tree conversion. The sidecar version is 9 and records that exact policy identity.

Reproduce into new directories from the repository root (PowerShell):

```powershell
cargo build --locked --no-default-features
New-Item -ItemType Directory -Force runs | Out-Null
$package = 'data/owned/poe2/3887ae68'
./target/debug/poe-optimizer.exe publish-owned-successor "$package/import/compiled/recipe.json" `
  --successor "$package/resistance/recipe.json" `
  --mapping "$package/import/compiled/mapping.json" `
  --roles "$package/import/compiled/roles.json" `
  --normalization "$package/import/policies/normalization.json" `
  --rewards "$package/import/policies/rewards.json" `
  --items "$package/resistance/items.json" `
  --item-source "$package/resistance/item-source.json" `
  --query-set "original-01=$package/import/queries/original-01.json" `
  --query-set "original-02=$package/import/queries/original-02.json" `
  --query-set "original-03=$package/import/queries/original-03.json" `
  --query-set "original-04=$package/import/queries/original-04.json" `
  --query-set "original-05=$package/import/queries/original-05.json" `
  --output runs/pre-tree-reproduction
./target/debug/poe-optimizer.exe extend-owned-tree-catalog runs/pre-tree-reproduction `
  --catalog "$package/tree/tree-catalog.json" `
  --policy "$package/tree/catalog-policy.json" `
  --output runs/current-reproduction
```

Both destinations must not exist. The first stage is regenerated rather than stored as another catalog. Compare the second directory's 20 files byte-for-byte with this directory, excluding this README. The `owned_tree_cli` acceptance tests reproduce this chain, repeat conversion and normalize all five originals. Existing exporter and resistance predecessor guards remain unchanged.

See the [tree converter](../tree/README.md), [import package](../import/README.md) and [resistance component](../resistance/README.md) for source evidence and remaining mechanics.
