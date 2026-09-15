# Class-dependent passive effect data

The Rust `compile-owned-passive-views` command converts explicitly reviewed complete
stat lists into owned rules. `policy.json` supplies source selectors, expected lists,
numeric effects and receiver programs. These seven physical nodes cover the views in
the original builds' saved specifications, with additional elemental-to-spell/minion
replacement contrasts. The other view-bearing nodes remain unconverted.

The rule compiler selects a matching class first, then primary ascendancy, then the
default list. It replaces the entire list. Native execution reads the authored character
through typed v7 predicates; source names and node keys are offline conversion inputs.
All seven reviewed views select Witch. The selected Sorceress in original05 must retain
node 4739's default 10% increased Spell Damage.

| Owned stat number | Meaning |
| --- | --- |
| 7473 | Increased spell damage contributions, percentage points |
| 7474 | Increased elemental damage contributions, percentage points |
| 7475 | Increased damage granted by the player to owned minions; contribution bucket and summed player property |
| 7476 | Granted modifier received by the declared Skeletal Sniper actor, percentage points |

Intelligence contributions reuse 7472. These four new IDs append to the attribute
compiler's 7472-entry successor; they cannot be allocated from the older 7469-entry tree
bundle. Stat definitions, values, affected nodes and receiver targets are data, not Rust
skill/build dispatch.

Two stat-owned receivers make the minion property explicit: a Player receiver sums
7475's Increase contributions, and an exact Sniper actor-slot receiver reads that Player
property into 7476. It does not create minions, bypass grant activation, add this bonus to
player spell damage, or calculate total minion DPS. Local modifiers, other minion slots,
auras/allies/Offerings and full action damage remain separate rules. Normal whole-request
and contributor-completeness checks still apply.

After generating `runs/owned-attributes-package` using
[the attribute command](../attributes/README.md), run from the repository root:

```powershell
$viewData = 'data/owned/poe2/3887ae68'
./target/debug/poe-optimizer.exe compile-owned-passive-views runs/owned-attributes-package `
  --catalog "$viewData/tree/tree-catalog.json" `
  --policy "$viewData/passive-views/policy.json" `
  --statistics "$viewData/passive-views/statistics.json" `
  --output runs/owned-passive-views-package
```

The destination must be new. The shared checked publisher preserves the ledger prefix,
physical nodes, topology, slots and all import/query policies. Only reviewed declaration
closures and previously unresolved programs are replaced. Additional/changed source views,
unlocks, ports or conflicting rules/receivers reject; an unchanged rerun reuses all IDs.
The result uses operation set v7; earlier v6 artifacts remain accepted unchanged by the
native compiler. Full original-build completion remains 0/5.
