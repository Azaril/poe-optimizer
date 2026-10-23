# Support target facts from definitions and exact occurrences

Status: investigated on 2026-10-22. Structural read authority remains a deferred
proposal; it is not a prerequisite for the current support corpus. An earlier
review prompt asked about that extension before the demand audit below finished.

The maintained release has support level/quality preparation rules, but no
production producers for the target facts required by `SupportInputBindingsInput`.
This prevents a complete evaluation bundle. Completing stage classifications alone
cannot fix it. The first repair is to stop demanding unused optional predicates,
then author the facts current supports actually consume. This proposal retains one
native rule and occurrence model.

## Immediate finding: unused inputs are demanded

No pinned PoB2 skill definition sets `supportGemsOnly=true`. Its only Lua mentions
are the generic CalcTools check, exporter plumbing and export database schema.
The five retained preparation artifacts contain 17 rows for 13 distinct supports;
every row explicitly declares `gems_only=false`, `from_item=false` and
`is_trigger=false`. Synthetic positive tests do not establish current game use.

`owned_supports.rs` already consults `has_gem`, `from_item` and `is_player_actor`
only when the selected support requires them. In contrast,
`owned_plan/supports/inputs.rs::read_target` demands all three first. Repair this
mismatch under the existing conditional semantics: preserve unknown unused facts,
and keep a consuming support unresolved when its required fact is unavailable.
`cannot_be_supported` remains universally consulted and needs real authority.
Preserve errors/work limits and meaningful causes when a fact is actually consumed.
Add computed-plan cases for both unused and required missing facts; the lower-level
preparation tests alone do not cover the current defect.

The bounded repair can remain private to the two computed drivers. Retain up to
four existing `UnavailableInput { key, cause }` values alongside the target, and
pass unknown Boolean facts through the existing three-valued preparation policy.
If its result is an applicability gap, use its exact `origin_index` and support
definition to select the relevant saved cause in a stable order. An unused flag
must not mask a required flag's cause. If no saved input explains the gap, retain
the existing preparation gap. Do not rerun preparation, swallow `PlanError`, or
change type/summoner, owner-coverage, schema, authority or freeze-stage validation.
Charge bounded origin lookup and diagnostic work. Share this adapter between
`owned_plan/supports.rs` and `owned_plan/support_effects.rs`.

Extend the existing `owned_computed_supports`, `owned_support_delivery` and
`owned_support_inherited_delivery` suites. Cover missing unused versus consumed
flags; a conclusive rejection despite an unknown conditional fact; two missing
flags where only the second matters; exact child versus parent failure identity;
effective-input failure precedence; exact/one-short budgets; and same-scratch
failure-to-success replay. Preserve `from_item && is_support`, rather than testing
item origin alone. A union of flags named by selected definitions would still be
too eager when another predicate conclusively rejects the candidate.

## Recommended ownership

| Fact | Owner and production source |
| --- | --- |
| Initial skill types | Reviewed Skill definition rules, before supports modify types |
| Optional minion types | Definition rules with explicit presence and complete membership within the declared vocabulary; absent and present-empty differ |
| Cannot receive supports | Definition prohibition where genuinely shared, plus separately proven restrictions on the exact supplying occurrence |
| Item origin | Checked exact provider/source relation, combined with any independently reviewed definition-side qualification |
| Player Actor | Existing Actor rule application and the Skill's resolved Actor relation; missing Actor coverage remains unknown |
| Summoner context | Existing exact receiving relation and the parent's prepared type result, without copying a frozen parent snapshot |
| Eligibility for Gem-only supports | A mechanical eligibility fact whose meaning must be established; neither physical Gem ownership nor PoB object presence alone proves it |

Initial type data and definition prohibitions fit the existing Boolean Stat/rule
model. Generated supplies can already project non-authored inputs. However, an
arbitrary authored direct Skill has no trusted structural source-fact producer.
Copying a source flag from imported settings would give user input authority over
a fact that should follow from the composed build.

## Decision: access to source provenance

**Recommend narrow structural rule reads over the validated composition graph.**
A Skill rule may request an explicitly allowed fact about its own exact source or
provider relation. The compiler checks receiver scope, relation authority and
dependencies; the native plan resolves the existing identity. Definitions and
rules still decide how that fact affects support eligibility. This is not an
arbitrary graph traversal API, a Lua field read, or a second source model.

The alternative extends mechanically produced occurrence inputs to authored roots
as well as generated roots. It reuses the current input projection shape, but
duplicates structural facts into slots and needs producer ownership, reconciliation
and invalidation for every composition change. Both approaches must reject
authored overrides and preserve unknown/incomplete facts.

The recommendation adds public read authority, so it needs design approval before
implementation, when an actual retained mechanic requires it. It does not approve
new source kinds, support sharing rules or
invented defaults. Reuse existing Actor rules and summoner receiving relationships;
do not add new read variants for facts those mechanisms already provide.

## Semantic issue to investigate before binding Gem-only eligibility

PoB's pinned `CalcTools.lua:108–133` checks `gemData` for Gem-only support admission.
The earlier [support-origin investigation](owned-support-origin-composition-proposal.md)
found `gemData` on manual Djinn occurrences that are not physical Gems. It also
found generated-only support exclusions that must not be placed on the shared
Skill definition. Therefore the current name `has_gem` does not establish the
native game meaning. Review the relevant support definitions, construction paths
and independent source controls before authoring a mechanical eligibility rule.
No retained support consumes this flag, so defer that semantic investigation until
a real definition needs it. If the evidence then establishes a PoB defect or
requires a new gameplay policy, bring that concrete choice back for review.
Missing evidence remains unavailable; physical Gem identity stays unchanged.

## Implementation and acceptance

1. Fix the unconditional demand for unused predicates and cover it at the computed
   plan boundary, without supplying false defaults or weakening required evidence.
2. Record initial definition types from pinned declarations. Existing independent
   seeds include `support-release/evidence.json` (Twister, Cleric, Heal) and
   `djinn-support-preparation/source-vectors.json`; final observed types cannot
   substitute for initial types.
3. If needed and after approval, specify the minimal structural predicates and authority checks
   needed by direct and generated occurrences. Bind them through the existing DSL
   and checked IR; do not add a separate interpreter or importer-only behavior.
4. Assemble actual support inputs with real level/quality producers in
   `migration-source/packets/group-100.kdl` and `group-101.kdl`. Complete the separate
   stage/readiness and source-relation inventories before publishing a bundle.
5. Preserve distinct results for physical, manual, item-generated and tree-generated
   uses of the same Skill, repeated copies, Player and supplied Actors, and changed
   equipment/tree selections. Compare parent prepared types for receiving skills.
6. Reject forged source facts and invalid receiver scopes. Test missing producers,
   absent versus empty type sets, deterministic parallel execution and invalidation
   after composition changes. No unchanged original becomes complete merely because
   these predicates are implemented.

Fixture defaults cannot resolve either the immediate demand bug or the deferred
source-authority question.
