# Reverse-primary source dispatch

One new maintained target: `crates/poe-optimizer-pob/tests/owned_generated_source_dispatch_source.rs`. Four tests; Rustfmt parsed; independent static review clear. No Cargo or Lua execution by the author. No existing source tests, production files, source archives or native schema were changed.

The target executes the same complete source boundary used by `skill_identity_extract::construct_catalog`: Data.lua prefix7–98 plus skill/Gem assembly910–1069, all nine actual skill modules, actual metadata initialization and complete Gem constructor loop. Every body is authenticated by the existing source manifest and every extracted span has an exact hash. It compares the full raw final Gem table to the complete constructed Gem table, and proves distinct actual Skill table identities before using source IDs as census keys. The expected retained catalog has966 final Gems,967 declaration occurrences and1436 final Skills; this target checks the final966/1436 sets. It does not reconstruct a partial role index as if that were a complete source census.

## Exact source semantics

- `Data.lua:979–1050` writes reverse lookup under the **primary effect table**, before appending effects and before display reordering. `1053–1066` runs constructors through `pairs(data.gems)`. Multiple Gems sharing a primary are order-sensitive; all candidates must be retained. A single VM's selected winner is not unique-source authority.
- `SkillsTab.lua:321–343` resolves an explicit saved game ID and variant first. `344–350` resolves a saved skill ID through its effect table and then the reverse lookup. The complete LoadSkill303–405 and ProcessSocketGroup1242–1322 execute in the new target.
- `CalcSetup.lua:1724` uses the same **table-keyed** lookup when constructing an item/tree-generated row. The test executes1690–1734 with an explicit host-supplied grant, then the actual ProcessSocketGroup. Acquisition, provider selection, support preparation and cleanup outside this span are not claimed.
- `SkillsTab.lua:1267` instead asks for `gemForSkill[gemInstance.skillId]`, a **string** key. A bare call with only skillId therefore takes direct fallback in this pinned source; that is distinct from the preceding saved/generated table-keyed dispatch. An explicit internal gemId takes precedence and an invalid explicit ID does not fall through.
- `ItemsTab.lua:4104–4116` uses table-keyed lookup for an item tooltip. `PassiveTreeView.lua:1853–1868` enumerates the reverse map for a granted-skill tooltip. Neither constructs the saved/generated runtime group.
- `SkillListControl.lua` contains no `gemForSkill` lookup in the pinned source. It displays/edits groups and opens a tooltip/wiki from existing row data. The relevant dispatch class is **SkillsTab**.

The source scan found these exact `gemForSkill` sites: Data972/982, SkillsTab347/1267, CalcSetup1724, ItemsTab4107, PassiveTreeView1858. The new target executes the constructor and three actual construction-path reads. Tooltip sites remain a read-only census, not extra simulated execution.

## Controls

1. Complete raw and constructed primary candidate maps agree in four fresh VMs (JIT off/on twice), with exact full key sets and unique Skill object pointers. Firebolt and each Djinn have one primary candidate. Both Commands have zero primary candidates while remaining actual additional effects. Firebolt's additional stat-set alias remains distinct from a second effect.
2. Actual saved inferred route, explicit game/variant route, generated route and bare ProcessSocketGroup fallback retain their different object identities and precedence. Generated source/slot/sourceNode objects are checked, but the host supplies their acquisition.
3. Complete zero-candidate Commands use direct construction. Deleting Firebolt's reverse-map entry yields the same direct shape while the complete candidate list still has one entry; missing map data is not zero-candidate evidence.
4. A synthetic second Gem sharing Sand Djinn's primary is constructed in both orders. Both candidates remain present; the observed winner follows construction order. Display reordering puts Command first without changing primary reverse identity. Explicit saved game/variant resolution still selects the original Gem.

Existing `owned_secondary_effect_switch_source` tests1427–1696 already cover actual Firebolt singleton/stat-set routing, exact source/slot/level replacement, saved duplicates, direct fallback count behavior, alias-induced second effects, independent selectors and zero/one filter transitions. `support/manual_direct_selector.rs` already covers both actual saved Djinn groups and their two-effect order. These tests remain unchanged; the new target avoids duplicating their full provider/selector matrices.

This is source-component evidence only. It grants no Import receipt, owned Skill identity, native activation, numerical equivalence or whole-build completeness. Any runtime census must join the exact source effect ID to the independently validated owned Skill and retain zero/one/multiple states; it must not encode a `pairs` winner as uniqueness.

Root owns execution:

```powershell
cargo test -p poe-optimizer-pob --test owned_generated_source_dispatch_source -- --nocapture
cargo clippy -p poe-optimizer-pob --test owned_generated_source_dispatch_source -- -D warnings
```
