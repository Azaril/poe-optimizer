# Ordinary attribute-passive effects

These are caller-supplied inputs for the Rust `compile-owned-attributes` data-build
command. They convert the reviewed finite tree catalog's ordinary attribute choices
into the existing owned rule format. No source checkout, Lua interpreter or example
build is required to compile the package.

`statistics.json` declares three Actor Integer contribution channels, allocated through
the existing registry in sequence: 7470 Strength, 7471 Dexterity and 7472 Intelligence.
`policy.json` maps the exact source lane/text to those IDs and their values. The command
checks the IDs against the supplied predecessor; these files cannot be applied to a
conflicting ledger. Production code contains no attribute name, coefficient or build ID.
The channels are flat contributions, not final effective attributes or requirements.

The compiler preserves all 293 physical nodes and their choice slots. It emits one
Choice/Compare/Contribute program per node, contributing only the selected lane's value.
It closes the reviewed port lists and program membership; topology, access, point costs,
class defaults, other passives and final attribute receivers remain separate obligations.
Additional source effects, class views, unlocks, unknown choices, incompatible schemas,
and conflicting existing programs reject conversion. An unchanged rerun is idempotent;
changing an already published rule needs an explicit mechanics update.

From the repository root (PowerShell):

```powershell
cargo build --locked
$attributeData = 'data/owned/poe2/3887ae68'
./target/debug/poe-optimizer.exe compile-owned-attributes "$attributeData/current" `
  --catalog "$attributeData/tree/tree-catalog.json" `
  --policy "$attributeData/attributes/policy.json" `
  --statistics "$attributeData/attributes/statistics.json" `
  --output runs/owned-attributes-package
```

The parent directory must exist and the destination must be new. The command validates
all predecessor artifacts and bindings, appends/reuses the explicitly supplied stat
IDs, compiles rules, then publishes through the shared atomic no-replace writer. Tree
normalization, item/reward policies and all five original query lists carry through the
same checked transition. The receipt names both schema endpoints and every passive
whose declaration closure changed. Native evaluation loads only the resulting owned
schema/rules/routing; source catalog and conversion policy stay offline.

The checked-in `current` bundle remains the 7469-entry predecessor. This compiler's
output is the 7472-entry successor for subsequent attribute integration work; regenerate
it rather than creating a parallel registry from the older import/resistance seeds.
No generated successor is checked in here. Complete native original builds remain 0/5.
