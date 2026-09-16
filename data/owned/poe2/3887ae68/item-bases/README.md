# Item-base identities and attack-profile source presence

This finite catalog covers all 1,756 final constructed item bases in the pinned upstream
revision. It records 337 explicit weapon tables and 1,419 absent weapon fields. The owned
capability means only **the template supplies a base attack profile**. It does not mean a
skill can attack with that item, that any numeric channel is complete, or that a concrete
item/equipment use is active.

The optional Rust exporter reuses the audited, bounded item-definition constructor. It
verifies the checkout and reads its final constructed table, so repeated assignments and
the actual module load list are honored. It publishes a small finite catalog without Lua
tables, callbacks, UI objects or build state. `catalog.json` carries the 29 base-construction
and interpretation source pins; `evidence.json` retains the 89 contributing constructor
pins and independent artifact/extractor hashes. Copied provenance is not authentication.
The exporter can compare a supplied catalog against its fresh authenticated result.

`policy.json` pins the exact catalog bytes and assigns each base an owned template ID and
exact source-header rule. `definitions.json` appends Capability 7483, then 1,755 templates
in canonical base-name order (7484–9238), reusing the existing Sapphire Ring template
2524 without changing its descriptor. The complete registry history remains one sequence.
These files are reviewed authoring data, not defaults inferred from a character build.

Every new template has partial equipment, modifier, quality and input-port membership.
Its item-level range is a portable nonnegative integer envelope, not a claim about legal
game levels. The source-presence programs retain Partial game-rule coverage. Unknown
field shapes have no Boolean producer. No quality, item-level or parameter absence is
inferred, and new modifier-index prefixes remain unresolved. Exact base headers can
identify a rare/unique item's base without declaring its title, unique effects, variants,
runes or other lifecycle semantics complete. Decorated magic names need a separately
reviewed resolver; substring matching is not used.

Reproduce the finite catalog with the optional reference-side adapter:

```powershell
cargo run --features pob -- export-owned-item-bases `
  --source-root vendor/path-of-building-poe2 `
  --output runs/item-bases-export
```

Generate the [intrinsic predecessor](../intrinsic-attack/README.md), then compile with the
default native Rust CLI. The converter does not load a checkout, Lua or legacy metadata:

```powershell
cargo run --no-default-features -- compile-owned-item-bases runs/owned-intrinsic-attack-package `
  --catalog data/owned/poe2/3887ae68/item-bases/catalog.json `
  --policy data/owned/poe2/3887ae68/item-bases/policy.json `
  --definitions data/owned/poe2/3887ae68/item-bases/definitions.json `
  --output runs/owned-item-bases-package
```

Destinations must be new directories with existing parents. The host budgets the checked
prior bundle and authoring inputs separately at 64 MiB each; constituent validation and
publication limits are unchanged. The checked successor host
preserves old definitions, import policies and the original 110 query rows. Reruns preserve
semantic artifact bytes. Source acquisition evidence is separate from the native package.

Next work must convert numeric base channels and local modifiers, then independently
establish selected hand/actor, skill compatibility, replacements and activation. Caster
bases and empty hands can select intrinsic sources only under reviewed action policy;
an incompatible martial weapon must never become an unarmed fallback. Minion actions
need their own actor data. Original04's selected Crossbow Shot with an active Ashen Staff
must remain an explicit incompatibility. Complete native original evaluations remain 0/5.
