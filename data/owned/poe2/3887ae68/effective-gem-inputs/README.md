# Effective Gem input preparation

This reviewed authoring fixture constructs a new full endpoint from exactly the
Cleric release input `ccaa8ceb07792a512fdde523c981b479b7a9c9a6292443cbc0ca4dd34aee22a8`.
It uses the public recipe compiler and full-release assembler. It does not extend
the append-only V1/V2 migration contracts. `authoring.json` contains the finite
binding facts and allocation preconditions; the Rust authoring test produces the
schema-bound `recipe.json` and complete `endpoint.json` deterministically.

The five bindings cover Twister, Skeletal Sniper, Skeletal Cleric, Meat Shield II,
and Elemental Armament II. Shared output stat identities address each exact Skill
or SupportOrigin occurrence independently. The only reviewed external family here
is global `GemProperty(keyword=minion,key=level)`: Sniper, Cleric and Meat Shield
read it; Twister and Armament do not. `evidence.json` records the physical metadata
and source observations. It contains no caller-authoritative effective inputs.

Active pre-support level is an unvalidated Count quantity: clamp raw level plus
corruption delta to at least one, then add external properties. Fractional values
survive. Support preparation performs the reviewed dense validation and returns
an integer. Active validation occurs later, after supported properties, and is
not implemented by this recipe. Explicit ordinary quality is required for every
real binding; absent or other quality remains unresolved, preserving Partial
quality membership.

| New ID suffix | Meaning |
| --- | --- |
| `30a9` | Required physical Twister corruption delta, Count |
| `30aa` | Required physical Sniper corruption delta, Count |
| `30ab` | Player-scoped global minion Gem level contribution, Count |
| `30ac` | Exact Skill pre-support level, Count quantity |
| `30ad` | Exact Skill pre-support quality, percentage points |
| `30ae` | Exact SupportOrigin prepared level, integer |
| `30af` | Exact SupportOrigin prepared quality, percentage points |

The old Twister/Sniper `primary-supply` programs retain their identities and grant
activation, but their raw level/quality projections and unused supporting nodes
are removed. Neither pre-support values nor raw values substitute for final
generated Skill parameters. Those required parameters remain unresolved until
the physical source-Gem post-admission bookkeeping is implemented.

Every old coverage declaration, query, source pin and provenance entry remains
unchanged. New support rule owners remain explicitly Partial. Actual item
contribution producers, other eligible property families, stage/activation
coordination and final active inputs remain incomplete. The new required Twister
and Sniper corruption slots deliberately have no normalization producer yet;
their old neutral guards are preserved and missing raw inputs are not zero.
All five original builds remain Pending; complete numerical coverage is 0/5.

Run the ordinary compact CLI tests with `cargo test --test owned_effective_gem_cli`.
To exercise the full real endpoint explicitly:

```powershell
$env:POE_OPTIMIZER_TEST_EFFECTIVE_GEM_PRIOR = 'runs/owned-cleric-abilities-02/package'
$env:POE_OPTIMIZER_TEST_EFFECTIVE_GEM_OUTPUT = 'runs/owned-effective-gem-release-01'
cargo test --test owned_effective_gem_cli explicitly_supplied_real_cleric_release_has_reproducible_preparation_successor -- --ignored --exact --nocapture
```

The output directory must not exist. The test checks the complete predecessor
against its immutable receipt, proves an exact change allowlist, compares CLI
fragments with the public compiler, publishes and rebuilds the complete endpoint,
checks byte-identical query files and rebuild, and freshly normalizes all five
originals. It never modifies the predecessor or protected allocation artifacts.

Validated on 2026-10-01 UTC: the two default CLI tests passed and the real test was
reported as ignored by default. The explicit real test then passed separately.
`runs/owned-effective-gem-release-01/package` and `rebuilt` contain identical
18-file, 58,387,813-byte releases, with seven appended allocations through `30af`.
The exact preservation assertion includes every previous rule, table, receiver,
schema declaration and policy after restoring only the documented edits and
dependency commitments. All five fresh normalizations remain Pending.

- New release input: `bb501a9181af6cf4c93828fb753a37cb107f3d61c26c0f82a5bf20ffac4245c0`.
- New definitions: `1939f2933da31b1c0963eacefc5a2602bbf58adb675ca84dfc564cad2d2a0b7d`.
- New registry: `17c99a5f4d5d4fb84c71b7f069929021c50ee1a21ba0324e1fdee3140a9bf666`.

The ignored run receipts are `validation.json` and `execution-receipt.json` in
that directory; logs are `runs/effective-gem-cli-tests.log` and
`runs/effective-gem-real-publication-tests.log`. Numerical build coverage remains
0/5. The release intentionally has no evaluation artifact group.
