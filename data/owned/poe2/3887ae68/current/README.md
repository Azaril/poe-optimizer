# Current owned package

This is the single published predecessor for new owned catalog work: registry watermark **2589**, with the complete earlier registry prefix retained. Use `recipe.json` and `mapping.json` here when adding definitions; do not allocate from the older 2515-entry import bundle. The first checked transition permits new schema addresses while retaining every previous definition and slot exactly. Refinement or retirement requires a separately reviewed transition policy.

Schema wire version 2 requires explicit class/ascendancy implicit roots and supports a point pool accepting both shared and scoped allocations. This bundle has been regenerated through the offline chain without changing registry history, rules, queries or source pins; it does not yet contain the converted class/passive catalog.

The 18 generated JSON files were emitted by the production `publish-owned-successor` command. It validates both recipes, compiles the successor rules, verifies append-only registry history and exact prior declarations, validates the old import bindings, and explicitly rebinds preserved import facts. The item policies are supplied successor-bound inputs. No PoB checkout or Lua execution is required.

`schema.json`, `rules.json` and `routing.json` are portable inputs for native semantic evaluation. `registry.json` is the authoring identity ledger. `mapping.json`, `roles.json`, `normalization.json`, `rewards.json`, `items.json`, `item-source.json` and the five `queries-original-*.json` lists belong to import tooling; native evaluation does not interpret those source policies. `recipe.json` assembles the owned packages for offline validation. `manifest.json` and `transition.json` record content identities and artifact hashes, not numerical completeness or exporter authentication.

All five original builds normalize through this package, retaining 116 item records, 478 gem records, 81 reward selections, 195 equipment uses and 110 ordered query rows. They remain partial drafts: **0/5 whole original builds completed natively**. In particular, modifier execution order is pending until source attribution proves it. The catalyst scalar is a component result, not a final resistance contribution.

Reproduce into a new directory from the repository root (PowerShell):

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
  --output runs/current-reproduction
```

The destination must not exist. Compare its 18 generated files byte-for-byte with this directory, excluding this README. The `owned_successor_cli` acceptance test performs that comparison and runs all five builds through the existing `normalize-owned` command using the emitted paths. Existing import exporter preservation checks and the resistance generator's exact predecessor guard remain unchanged.

Source provenance and the intentionally incomplete mechanics are described in the [import package](../import/README.md) and [resistance component](../resistance/README.md).
