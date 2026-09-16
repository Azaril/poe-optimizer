# Class base attributes

`policy.json` binds three reviewed source integer fields to existing owned contribution
stats 7470 (Strength), 7471 (Dexterity) and 7472 (Intelligence). The Rust converter reads
all eight class rows from the caller's pinned tree JSON. No class values, names, example
builds or Lua execution are built into the converter or native evaluator.

Generate the attribute and passive-view predecessors first, following
[their instructions](../passive-views/README.md), then run from the repository root:

```powershell
./target/debug/poe-optimizer.exe compile-owned-class-bases runs/owned-passive-views-package `
  --source-tree vendor/path-of-building-poe2/src/TreeData/0_5/tree.json `
  --policy data/owned/poe2/3887ae68/class-bases/policy.json `
  --output runs/owned-class-bases-package
```

The parent directory must exist and the destination must be new. Source acquisition stays
offline; consumers need only the published owned package. Source pins, complete class
membership, reviewed field shape, target types and predecessor bindings are validated.
All 7476 registry entries, physical roots, class/ascendancy relations and 110 query rows
are retained. An unchanged rerun reuses the same definitions and programs.

Only reviewed input declaration closures become complete. Class numerical rule coverage
stays partial because class-specific unarmed defaults and any remaining intrinsic effects
are not represented by these three fields. Class-base contributions are not complete
attributes. The full [attribute design](../../../../../docs/owned-attributes.md) records
finite staged dependencies, MORE grouping, rounding and downstream inherent bonuses.
Complete native original builds remain 0/5.
