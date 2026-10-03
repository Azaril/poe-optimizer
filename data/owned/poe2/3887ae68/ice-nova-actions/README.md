# Ice Nova action correspondence

This packet declares the physical Ice Nova primary and its two constructed
stat-set alternatives. It does not implement Ice Nova damage, infusion behavior,
support effects, final skill inputs or complete physical input inventories.
Publication passes in `runs/owned-ice-nova-actions-04/validation.json`;
`authoring.json` requires this external publication receipt and records the exact
predecessor and authenticated source evidence.

The schema5/operations17 V3 migration promotes existing Skill `01df` to Known
with Partial declarations. Existing Gem `07dd`, corruption slots `30c0`/`30c1`,
quality knowledge and all prior Partial closures are preserved. The seven new
allocations are:

| ID suffix | Meaning |
| --- | --- |
| `327e` | Gem-owned primary supply of Skill `01df` |
| `327f` | Gem-owned entering grant for that supply |
| `3280` | Skill-owned player action output |
| `3281` | Singleton ordinary action part |
| `3282` | Singleton ordinary player action mode |
| `3283` | Constructed stat set 1, Ice Nova |
| `3284` | Constructed stat set 2, Cold-Infused |

The only new program activates the declared primary grant. Ordinary provider,
physical SkillUse and preset activation still apply. The supply and output
declarations identify potential topology; they do not establish numerical
execution readiness. The output has exactly the reviewed part, mode and two
stat sets. Its unreviewed choices and the containing Skill/Gem inventories stay
Partial. There are no new final-level or quality slots, numeric programs,
tables, receivers, routes or evaluation bundle.

`correspondence.json` records the source-to-owned addresses and five mapping
rows. The primary output and singleton part/mode are explicitly reviewed
correspondences. The stat-set mappings use the constructed scopes
`ice_nova_statset_0` and `ice_nova_statset_1`; the accompanying source index and
label are evidence, not runtime identity. `IceNovaPlayerOnFrostbolt` and
`IceNovaColdInfusedPlayer` remain metadata aliases. They are not extra physical
effects, standalone Skills, or an inferred alias-to-table ordering.

The `source_decoding` record supplies the shared typed index recipe and explicit
absence correspondence to constructed table1. The separately checked native
Import adapter binds this data to the published release before interpreting
`StatSetIndex` or `StatSetCalcsIndex`. Present malformed/missing indices and
duplicate rows remain unresolved. Legacy scalar headers are retained as source
evidence but overwritten by the pinned loader. MAIN/CALCS remain reference
contexts outside native game state. See [the adapter contract](../../../../../docs/owned-source-actions.md).

All 110 original queries are preserved. Original05's selected Ice Nova is the
physical occurrence at source ordinal239, saved SkillSet4/group11; the original
requested damage action remains Sniper. Archived Ice Nova copies in sets5,6,1
remain independent occurrences. Source ordinals are fixture evidence only and
are not embedded in the production declarations or mapping rules.

## Evidence and numerical boundary

The existing ignored test
`complete_source_spell_stat_sets_preserve_constructed_correspondence` in
`crates/poe-optimizer-pob/tests/owned_spell_stat_set_inputs.rs` passed both JIT
modes. It covers the baseline, thirteen controls and a repeated baseline. Each
report is 2,236,787 bytes with SHA256
`3da1b32a4ed08336ea603b5c85190252ab63aa95e3d40ebf5d5c8869eecd2326`.
The reports remain local at
`runs/owned-spell-stat-set-source-01/source-jit-{off,on}.json`; the authoring
receipt authenticates both files, the source manifest, catalog and pinned files.

The witness proves exact constructed table membership and physical ownership,
including independent contexts and duplicate physical sources. It reports
`has_parts=false`, `has_global_effect=false`, and original table1 selection.
It does not prove complete native action coverage or numerical parity. This
packet does not choose a canonical cold versus rebuilt reference lifecycle.

The two stat sets have different level vectors and modifiers. Source stat
assembly adds duplicate stat names and duplicate constants; these cannot be
converted using last-field replacement. The selected level row overlays a copy
of common level data. Source processing merges selected local modifiers and
global modifiers from other sets, while ordinary quality adds conditional hit
damage against chilled enemies. The Cold-Infused set also requests average
damage presentation. None of these behaviors is implemented by declaring two
stat-set IDs.

Future numerical conversion must supply supported final level/quality,
stat-set-specific rules, exact modifier contributors and conditional inputs
through the existing native contracts. Observed level17 or any observed damage
is not a production default. Complete physical parameters, saved usage,
support preparation and whole-build coverage remain Pending or Partial.

The publication helper must authenticate the predecessor and source receipts,
apply the checked migration and mapping append, reproduce artifact bytes, and
prove that all unrelated data and original queries survive unchanged.
