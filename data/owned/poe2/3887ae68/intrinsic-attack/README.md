# Intrinsic attack baselines

These artifacts describe the numeric attack baseline supplied by each class. They are
independent of hand occupancy, martial-weapon eligibility, action activation and the
selected source for a particular action. The conversion preserves partial Class rule
coverage. Complete native original-build evaluation remains 0/5.

`catalog.json` is finite data exported by the optional Rust/`mlua` adapter from reviewed
source bytes. It retains nine rows and all five fields. `policy.json` pins its exact bytes,
explicitly excludes source class 0 from this eight-class namespace, checks the `type`
constant and maps every numeric field to an owned Actor stat and exact unit. Source pins
are carried provenance; the separately reviewed catalog digest checks artifact identity.
Native evaluation receives only the resulting owned literal Derive programs.

| Registry sequence | Owned meaning | Exact unit |
| --- | --- | --- |
| 7477 | Intrinsic attack rate | 7481: attacks per second (Rate) |
| 7478 | Intrinsic critical chance | 2: percentage points |
| 7479 | Intrinsic physical minimum | 7482: damage (Damage) |
| 7480 | Intrinsic physical maximum | 7482: damage (Damage) |
| 7481 | Rate unit | New owned unit |
| 7482 | Damage unit | New owned unit |

The six definitions append to the 7476-entry class-base successor. No independent ledger,
synthetic equipment or hard-coded runtime class branch is created. Each exclusive class
program supplies rate 1.65, critical chance 5 and minimum 2; maximum is 5, 6 or 8 according
to the injected row. These values are source data, not total character attack statistics.

To reproduce the finite catalog from the optional checkout:

```powershell
cargo run --features pob -- export-owned-intrinsic-attack `
  --data-lua vendor/path-of-building-poe2/src/Modules/Data.lua `
  --misc-lua vendor/path-of-building-poe2/src/Data/Misc.lua `
  --source-pin data/owned/poe2/3887ae68/intrinsic-attack/source-pin.json `
  --output runs/intrinsic-attack-export
```

The exporter checks both complete source files against the reviewed upstream manifest
before evaluating their finite data, disables JIT, restricts environments and applies
memory/instruction/table limits. It never constructs the PoB UI or evaluates a build.
New upstream revisions require reviewing the adapter and updating the source manifest.

Generate the [class-base predecessor](../class-bases/README.md), then publish with the
default native Rust CLI, without Lua or a PoB runtime:

```powershell
cargo run --no-default-features -- compile-owned-intrinsic-attack runs/owned-class-bases-package `
  --catalog data/owned/poe2/3887ae68/intrinsic-attack/catalog.json `
  --policy data/owned/poe2/3887ae68/intrinsic-attack/policy.json `
  --definitions data/owned/poe2/3887ae68/intrinsic-attack/definitions.json `
  --output runs/owned-intrinsic-attack-package
```

Both destinations must be new directories with existing parents. Publication uses the
shared checked successor path and preserves the original 110 query rows, import policies,
tree topology and all previous definitions. Repeated conversion is idempotent. A selected
class contributes the baseline; action routing must still establish whether an action uses
it. Occupied caster weapons and empty hands must remain distinguishable. See the
[attribute and source-selection design](../../../../../docs/owned-attributes.md).
