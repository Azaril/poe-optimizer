# Authored item header inputs

These injected item-line/source policies replace the prior opaque `Quality:` and
`Item Level:` metadata rules with explicit values. They are bound to the owned
current local-weapon schema `c15b995b5b31fda9bbe3454aeff5eaeeb03fa4d5dae9ead0fe8e7ad16c32b90a`
(definition schema 7, rebuilt during the generic Actor migration).
The source policy remains v3. Existing line rules, source layout evidence and all
other semantics are preserved.

The admitted spelling is the exported unsigned integer header: `Quality: 20`,
`Quality: 0` or `Item Level: 82`. Quality selects owned standard-quality kind 6,
with percentage-point unit 2. Item level is an integer checked against the exact
selected template. Missing, malformed, duplicate and inapplicable selections do
not produce an invented value. The numerical envelope is a representation bound,
not proof that an in-game item is obtainable.

This connects explicit imported weapon quality to the 337 template-owned quality
readers and four local numerical recipes. It does not complete item modifier
membership/order, source range indexing, rune reconstruction, catalyst/magnitude
processing, crafted-quality state, action routing or whole-build evaluation.
Template quality applicability is still supplied by the definition package.

The two policy files are full versioned inputs to the existing validators. Their
meaning lives in data, with no build/skill/item-name dispatch in the CLI or engine.
The source policy carries the exact item-line digest; neither binding is silently
rewritten when supplied. The empty recipe extension deliberately allocates no new
schema or calculation data.

Publish after rebuilding the local weapon recipe checkpoint in the current
format. Set `$LocalWeaponRelease` to that rebuilt package; archived schema-2
packages are reference evidence and are no longer accepted runtime inputs.

```powershell
cargo run --release -- extend-owned-recipe $LocalWeaponRelease `
  --extension data/owned/poe2/3887ae68/item-header-inputs/extension.json `
  --items data/owned/poe2/3887ae68/item-header-inputs/items.json `
  --item-source data/owned/poe2/3887ae68/item-header-inputs/item-source.json `
  --output runs/owned-item-headers-01/package
```

The output must be a new directory. A different predecessor schema requires
explicitly authoring and validating a new bound pair. All five supplied build
imports and all 110 reference query rows remain in the integration test; the
header change does not declare any original build complete.
