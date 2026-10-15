# Selected configuration constructor supplier review

This offline checkpoint excludes the fifteen Mana adjustment supplier names
listed in `review.json` from the selected literal configuration callback bodies,
the selected inactive boss-skill branch, and the empty custom-text paths. It does
not grant native coverage, complete Mana, retire imported configuration leaves,
or infer missing sources from zero-valued runtime outputs.

`configuration-review.json` is reconstructed by the Rust test
`owned_mana_configuration_sources`. It uses the existing authenticated source02
acquisition and exact selected-input reconciliation. The `off` and `on` labels
identify the two retained reports and their plain control modes. Both observed
constructor traces were captured with JIT disabled; the independent plain
controls covered JIT off and on. No additional PoB execution was used here.

## Bodies and observed occurrences

The original catalogue has 663 rows, including 537 callback declarations and 126
rows without a callback. Default construction invokes 62 callbacks; loading the
selected saved configuration invokes 63. The other 475 and 474 callback
declarations respectively have a checked false constructor dispatch gate. UI
visibility metadata such as `ifSkill`, `ifOption`, and `ifMult` does not suppress
this dispatch and is not used as exclusion evidence.

The review checks all 125 actual invocations in catalogue order, exact callback
file and line identities, arguments, complete before/after state chains, and
append-only modifier deltas. It reconstructs the first configuration's 45 input
defaults and 19 placeholder defaults from the complete catalogue. All original
records remain in the authenticated acquisition; the new report references
each delta by digest and preserves occurrence counts.

`configuration-bodies.json` retains 48 complete original callback spans:

- 45 bodies have reviewed literal writer-name domains, including nested
  `MinionModifier` records. Every emitted record must belong to the body's
  declared domain. Their declarations and actual records contain none of the
  fifteen target names, including zero or conditionally inactive candidates.
- `presetBossSkills` has a dynamic writer branch. Its actual argument is `None`
  in both constructions, and the exact complete body excludes that branch.
- Two quest parser callback bodies cover 17 distinct quest variables. Their 34
  actual deltas are retained and screened for the target names, but the parser
  and captured reward-string authority remain an explicit unresolved boundary.

Both constructions have exactly one enabled, empty custom block and no fallback
custom text. The full `BuildModList` body proves that neither parser path runs.
The final default configuration contains 52 Player and 21 enemy records; the
saved configuration contains 66 Player and 21 enemy records.

## Source authority and helper calls

The complete `ConfigOptions.lua` and `ConfigTab.lua` archives are reused from
`../configuration-accounting/evidence/pob/`. Their paths and normalized source
hashes must match the authenticated upstream manifest. Exact callback and
constructor excerpts are checked against those complete original sources.

`configuration-auxiliary-bodies.json` retains the complete original
`EditControl.lua` and `DropDownControl.lua` sources, pinned to the same manifest,
plus the reviewed helper excerpts. `SetPlaceholder(text, true)` calls the
numeric configuration control with `placeholder=true`; the corresponding
`ConfigTab` callback updates that placeholder and marks the build dirty. It
does not take the authored-input branch that recursively calls `BuildModList`.
The selection helper only updates its selected index. Arithmetic and display
details are original-source evidence, not new native runtime behavior.

These upstream source copies and excerpts retain their source headers and are
covered by `upstream-LICENSE.md.gz` in this directory and the readable license at
`../configuration-accounting/evidence/pob/LICENSE.md`. Archived
source text is never executed by the ordinary proof.

## Remaining boundaries

The quest-string parser boundary is not closed merely because these selected
outputs contain no target names. Configuration control records and raw fields
can affect later calculation bodies, so those consumers still need their own
review and native input adoption. In particular, rows with no `apply` callback
are not necessarily presentation-only: `enemySpeed`, `enemyCritChance`, and
`enemyCritDamage` are read directly by defence calculations.

This checkpoint changes no `01f2` configuration responsibility, owner completion,
or native contribution membership. The broad supplier proof still has the
other constructor and later delivery domains listed in `review.json` open.

Mutation tests reject missing catalogue/callback rows, wrong file identities or
arguments, changed defaults and body declarations, prior-record rewrites,
nonempty custom text, and nested zero/inactive target records. A separate finite
dispatch test preserves numeric zero and tests the count-only fallback rule;
it is confined to offline source evidence.
