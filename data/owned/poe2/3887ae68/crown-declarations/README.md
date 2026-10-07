# Iron Crown intrinsic declarations

This packet refines exactly five empty intrinsic declaration inventories on
ItemTemplate `1f1c`: choices, grants, actors, skill grants and outputs. The existing
six-parameter inventory is unchanged. The socket declaration remains Partial:
the exact base has `socketLimit = 3`, and the selected original item retains three
empty socket groups. Empty groups do not mean the host has no socket inventory.

The predecessor is `runs/owned-sand-preparation-publication-03/package`, input
`9bbb63fab97fca1b60a67ccb048103e51677a9b0668a0e9d130355b0f930cd24`.
The V5 migration replaces one descriptor using the existing schema6 / operations22
contract. It allocates no definitions, adds no programs and changes no query.
All seven Crown programs, their Partial owner closure, the Partial quality and
modifier memberships, and every unrelated descriptor remain unchanged.

## Authority and evidence

The full catalogue base has Armour/Energy Shield, requirements, default quality,
tags and socket capacity. It has no intrinsic choice, grant, actor, generated skill
or exposed output declaration. An empty intrinsic output inventory does not mean
the item has no calculated stats: the existing local-defence and other programs
are preserved. Pinned `Item.lua:2800–2808` constructs `grantedSkills` from the
assembled `ExtraSkill` modifier records. Modifier-owned grants therefore remain
the responsibility of modifier definitions and their still-open inventories.
The selected item's lack of such records is corroboration, never a general
absence law for possible rolls or modifier-created skills.

No new source VM run is needed. `source-vectors.json` joins three existing,
independently hashed JIT-off/on report pairs:

- `owned-armour-local-defence-source-03`: the complete exact Iron Crown base,
  original item identity and saved selection across fresh/rebuild-one/rebuild-two,
  independent replay and a restored warm control. Observed and unhooked projections
  must match. The source identity joins item21 to original05 source ordinal576.
- `owned-armour-item-inputs-source-01`: the original loaded item, fresh construction
  before and after `BuildModList`, reparse, and another fresh construction. The
  complete captured source-category lists retain the actual Minion-level property
  and three socket groups, with no `ExtraSkill` record in those constructions.
- `owned-extra-stat-consumption-source-05`: every captured environment's exact
  selected Helmet object and empty `grantedSkills`, across MAIN, CALCS and
  CALCULATOR, three lifecycle stages and independent replay. The full-report gate
  also compares uninstrumented numerical outputs. No supplier-domain completeness
  or native coverage authority is inferred from this source report.

The ordinary Rust tests authenticate the committed projection, source manifest,
local observer/test/fixture pins, intended descriptor inverse and negative
identity/grant/socket controls. Publication additionally authenticates the full
retained reports and pinned source modules, reconstructs every projection, and
uses the shared all-five normalization/finalization and byte-identical rebuild
checks. Authoring evidence stays outside the published native package.

## Validation

Both ordinary Rust tests pass. Publication01 passed in 27.64 seconds, producing
`runs/owned-crown-declarations-publication-01/package`, input
`5d8757d9046ef837509223103f06931fb16aac7a3b60aa8231b6f48b5f4225aa`.
The full retained-report authentication, exact migration inverse, all-five
preservation checks and byte-identical rebuild pass. Selected issue counts remain
`107/117/109/123/5`; no evaluation bundle or complete original build is added.

All four finite real-item-to-Sand integration tests also pass against this
successor in 13.34 seconds. They execute the actual Crown/Solar Minion-level
programs and canonical inputs within the explicitly bounded component; they do
not certify the items' remaining mechanics or whole-build contributor coverage.

```powershell
cargo test -p poe-optimizer-cli --test owned_crown_declarations --locked
$env:POE_OPTIMIZER_TEST_CROWN_DECLARATIONS_PRIOR='runs/owned-sand-preparation-publication-03/package'
$env:POE_OPTIMIZER_TEST_CROWN_DECLARATIONS_OUTPUT='runs/owned-crown-declarations-publication-01'
cargo test -p poe-optimizer-cli --test owned_crown_declarations publish_crown_declarations_preserving_all_five_originals --locked -- --ignored --exact --nocapture
```

Use a fresh output directory. The exact migration inverse and dependency checks
must retain the socket gap, all numerical owner gaps, all modifier/quality gaps,
the registry, queries, original sources, and their existing unresolved issues.
