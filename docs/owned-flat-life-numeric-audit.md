# Canonical Life numeric boundary audit

Status: source and native regression evidence, 2026-10-09. No production rule,
schema, import policy or closure changes. The broad numeric gap remains open.

The current flat-Life owner has continuous raw-amount and corruption-factor
domains from zero through 1,000,000. Its fixed-integer importer admits a narrower
set. Proving those imported values cannot certify every canonical numeric input.
The existing compiler already lowers the numeric stages to ordinary typed rules;
the audit executes the actual released `effective-amount` program, not a second
Rust implementation of its formula.

## Boundary ownership

PoB `Item.lua` strips source annotations, selects ranges and constructs modifier
records. Those encoding and history questions belong to Import. Native rules
consume the admitted unrounded component, explicit initial factor and the final
ordered magnitude factor. The data compiler expresses internal rounding,
corruption, magnitude and final numeric rounding in sequence. Source text and
cached parser records are not native evaluation inputs.

Life's actual pinned scalability row has one scalable component and no format
tags. `ItemTools.lua:45–58` therefore uses precision one and the default final
formatting path. The audit calls that original function after full source loading,
with no method replacement or debug hook. It verifies that original Life output
is unchanged. Five originals, fresh replay and warm restoration agree with JIT
off and on. This is numeric component evidence in several loaded contexts, not
complete-build parity or a census of obtainable rolls.

## Observed decimal transport discrepancy

Sixteen controls agree, including zero/identity, adjacent half values,
fractional raw amounts, non-identity corruption, corruption before magnitude,
negative magnitude, zero factors and upper raw/corruption bounds.

The seventeenth control supplies raw `999999`, corruption `999999` and magnitude
`1001`. Native arithmetic produces `1000997998001001`. PoB's formatter returns
`1.000997998001e+15`, which parses as `1000997998001000`. The original numeric
rounding helper retains the first integer; `tostring` shortens it, while
`string.format("%.17g", value)` preserves it. The difference is repeatable in all
tested source contexts and both JIT modes.

These are diagnostic canonical inputs, not an obtainable-item claim or a change
to any supplied build. This finding is not an owner-approved PoB defect exception.
Keep the exact discrepancy visible; do not add Lua's decimal serialization to
the native evaluator, relax tolerance, narrow a schema to make tests pass or
retire the broad owner gap. Reachability and semantic bounds of real magnitude
producers still need evidence before deciding whether this matters to game
results. Those producers are the useful next mechanics dependency; the current
package has no `ProjectModifierTransform` effects.

## Evidence and reproduction

- Full-source audit: one passing regression in **26.79s**, including the exact
  retained discrepancy, eight complete loads per JIT mode and unchanged owner
  Partial status. Log: `runs/owned-flat-life-numeric-source-05.log`.
- Both complete reports in `runs/owned-flat-life-numeric-source-03` are 32,910
  bytes, SHA-256 `7ce49ac31129402864671b9f80d20049793ef437f276e191bfe690423f43d064`.
- Native replay: two passing tests in **0.22s**, exact released-program identity,
  all 17 controls, fresh/reused/reversed order, each missing input and a four-thread
  Rayon pool. Log: `runs/owned-flat-life-numeric-native-01.log`.
- Strict all-feature Clippy for both targets passes in **0.37s**.

The compact [fixture](../tests/fixtures/owned-flat-life-numeric.json) commits the
source observations, witness/driver/source identities and the actual native
program hash. Expected outputs are test assertions only. The normal native test
loads the existing public-input replay and requires that exact program body.

```powershell
cargo test --locked -p poe-optimizer-cli --test owned_flat_life_numeric
$env:POE_OPTIMIZER_TEST_LIFE_NUMERIC_RELEASE='runs/owned-flat-life-admission-publication-01/package'
$env:POE_OPTIMIZER_TEST_LIFE_NUMERIC_SOURCE_OUT='runs/owned-flat-life-numeric-source-03' # fresh path
cargo test --locked -p poe-optimizer-pob --test owned_flat_life_numeric -- --include-ignored
```

Canonical package and selected input obligations remain unchanged at
`26c0a5022e550ae9f2d2488c27b761a58917f2153aeda4b402f465a7d7c0bf76` and
`107/117/109/123/4`. Complete native builds remain 0/5. The owner has separately
approved exact Action queries and optional numeric selection; implement Action
authority first to resume the closest build's Command/damage consumer.
