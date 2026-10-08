# Ordinary Player Life routing

The flat-Life writer `3100 / contribute-player-flat-life` now requires the
existing EquipmentUse applicability channel `3306`. This is the same reviewed
Necromantic Talisman exclusion used by ordinary Minion-level delivery. It is a
typed data edit, with no evaluator branch, new definition or Lua runtime.

The predecessor is `runs/owned-gem-support-domains-publication-03/package`, input
`f3d994ac40afb840606c84a10df7e84fb60bcc216ac5700f2cb07e9a5a955c68`.
`owners.json` contains five exact before/after owners. Only the Life delivery
guard and four appended template programs change. Numeric preparation, effect
identity, recipient, amount, query membership, import admission and every owner
gap remain unchanged. The whole-input inverse checks this boundary.

The four new template predicates are Sapphire Ring, Fine Belt, Rope Cuffs and
Tattered Robe. Their authenticated source types are non-Amulet, so this specific
predicate is true. The existing Solar/Lapis Amulet predicates read Player
retention; the existing allocated Talisman producer supplies zero. Other
templates retain their existing state. Template identity comes from injected
data, never an imported build ID, display name or runtime English-text test.

## Evidence and limits

The pinned `CalcSetup.lua:1397–1403` branch diverts **every** source Amulet
modifier to the minion store before ordinary Player delivery. It has no stat-name
condition. Reuse its existing 21-case, three-stage, both-JIT original-call routing
certificate and the complete source-type catalog; do not create a second
observer or parser. Publication authenticates both retained reports and pinned
source files. This is a source-law transfer to Life, not a new measured
whole-build Life parity result.

Channel `3306` currently proves only the Talisman exclusion. Its broad historical
name does not establish all item routing. Copying Amulet bonuses, Focus scaling,
weapon substitutions, other item transformations and minion receipt still need
their own rules and coverage. In particular, suppressing ordinary Player Life
does not yet deliver that Life to a minion. The original six flat-Life owner gaps
remain; no complete equipment Life reduction or final Life formula is claimed.

The joined native fixture keeps the actual imported four Life-bearing records
and five equipment uses. It also adds a controlled Life modifier to an Amulet,
selects the actual published Talisman program in a finite passive component and
checks exact inactive/unchanged effects, removal, reusable scratch and Rayon.
Those controlled selections are tests, not a production admission shortcut.
Missing applicability must stay unresolved and early reads must fail staging.
The Life membership fixture restores only the authenticated donor edit when
checking its historical census; execution uses the new actual writer.

## Validation

The ordinary authoring check passes. Publication in
`runs/owned-flat-life-routing-publication-02` passes in 20.63 seconds, preserving
all five originals, all 110 queries and selected input counts
`107/117/109/123/4`, with a byte-identical package rebuild. Whole native builds
remain 0/5. All 79 joined native checks pass in 39.03 seconds against the first
publication, whose 18 artifact hashes exactly match the current publication.
Strict Clippy passes for both integration targets with all features. Results and
the remaining blockers are recorded in the
[living implementation checkpoint](../../../../../docs/implementation.md).

```powershell
cargo test --locked -p poe-optimizer-cli --no-default-features --test owned_flat_life_routing
$env:POE_OPTIMIZER_TEST_FLAT_LIFE_ROUTING_PRIOR='runs/owned-gem-support-domains-publication-03/package'
$env:POE_OPTIMIZER_TEST_FLAT_LIFE_ROUTING_OUTPUT='runs/owned-flat-life-routing-publication-02'
cargo test --locked -p poe-optimizer-cli --no-default-features --test owned_flat_life_routing publish_life_routing_preserving_all_five_originals -- --ignored --exact
$env:POE_OPTIMIZER_TEST_SNIPER_ITEM_ATTACK_RELEASE='runs/owned-flat-life-routing-publication-02/package'
cargo test --locked -p poe-optimizer-cli --no-default-features --test owned_sniper_item_attack -- --include-ignored
```
