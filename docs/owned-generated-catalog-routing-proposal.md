# Proposal: explicit catalog reverse-routing facts

**Status:** Proposed; no production implementation or authorization assumed.
**Date:** 2026-10-20.
**Decision owner:** Project owner.

The generated singleton receipt should consume a compiler-derived fact about
every catalog candidate for its primary, rather than infer uniqueness from the
roles supplied by a caller. Keep that fact in the existing Import-only
construction artifact. This is a narrow extension of the accepted construction
design, but adds public serialized semantics and needs an explicit scope decision.

## Existing boundary and concrete omission

`OwnedSkillRoleIndex::new` checks `roles.len() == compilation.gem_count`, then
checks each retained row against mappings and schema. Both the rows and count
come from its input. It does not require every mapped Gem to have a role or
receive the source catalog.

`verify_owned_skill_catalog_reproduction` does establish completeness against
the actual supplied catalog: exact catalog digest, both counts, every final
catalog key, complete role equality and no extra keys. `owned_catalog_recipe`
calls it. Ordinary `assemble_owned_release`, CLI normalization, and
`normalize_fresh` do not receive or require that reproduction result. The
accepted construction proposal intentionally distinguishes structural runtime
validation from offline source reproduction.

Therefore a structural role package can omit a colliding primary and adjust
`gem_count` while preserving the original catalog commitment, mappings, schema
and surviving role. A scan then sees one primary even though the authenticated
catalog contains two. The second candidate may have another effect. Checking
all *present* rows, or requiring all present constructions to be Observed,
does not fix this.

The maintained `structurally_valid_role_subset_cannot_prove_reverse_primary_uniqueness`
regression in `crates/poe-optimizer-import/tests/owned_skill_catalog.rs` captures
that counterexample using the existing compiler fixtures. Structural loading
accepts the subset and offline reproduction rejects it. The regression and
strict target lint pass in
`runs/generated-catalog-omission-regression-{tests,clippy}-01.log`; production is
unchanged. The investigation draft and exact source pins remain under
`runs/generated-catalog-completeness-audit-20261020/`.

## Source object identity, aliases and the evidence boundary

PoB's `Data.lua:981–982` resolves `data.skills[gem.grantedEffectId]`, then
uses that actual effect table as the reverse-map key. Grouping nominal strings
would be unsound if two distinct Skill IDs could refer to the same object.

Current acquisition rejects that domain rather than silently merging it:

- `skill_identity_extract.rs:545–547` visits every constructed `data.skills`
  entry and requires its table's raw `id` string to equal the entry key.
  `required_text` uses `raw_get` at lines 123–132. A shared table cannot have
  two different raw `id` values during this fixed observation.
- `skill_preparation_extract.rs:387–400` independently rejects duplicate
  effect-table pointers under different IDs. Lines 464–480 translate the
  actual table-keyed reverse map through those checked identities.
- `game_data.rs:525–527` runs these acquisitions together. The existing
  `SkillIdentityCatalog::gems_for_effect` query represents all nominal primary
  candidates; it explicitly does not choose the `pairs`-order winner.

Thus the ID-based projection is justified for an authenticated catalog acquired
through these checks. `SkillIdentityCatalog::new` on arbitrary JSON does not
independently establish Lua object identity; neither would the proposed new
field. Keep source reproduction and its acquisition provenance mandatory at
the offline publication boundary.

Add acquisition controls for two Gem keys sharing one primary object, and for
two Skill keys illegally sharing an object: the first retains both candidates;
the second must fail acquisition. If a future source permits aliases that
should be supported, extend offline acquisition to retain the complete
reverse-route observation or explicit alias relation before enabling this
projection. Do not infer distinct objects from distinct text IDs in that case.

An additional stat-set reference is a different source relation. It does not
cause another `gemForSkill` insertion or another constructed active effect.
Retain a separate Firebolt stat-set-alias control, with no fabricated candidate
or active-effect membership. None of these source object checks enter native
Actor, Skill or evaluator semantics.

## Recommended minimal data shape

Add one required field inside `ObservedGemSourceConstruction`:

```rust
#[serde(deserialize_with = "required_option")]
pub primary_catalog_candidates: Option<Vec<String>>,
```

Meaning:

- `None` is an explicit unavailable full-catalog census. It grants no new
  generated-routing field authority.
- `Some(keys)` is the complete set of **final catalog Gem keys** whose source
  `primary_effect_id` equals this row's source primary effect ID. It is not a
  list inferred from serialized roles, source selectors, or native Skill IDs.
- Keys are sorted and unique for deterministic set serialization, nonempty and
  include this row's `catalog_gem_key`. Ordering does not choose a source winner.
  Two keys remain ambiguous even if both map to the same native Skill.
- The existing enclosing compilation `catalog_digest`, source pin and policy
  bind this observation. Do not repeat these commitments per row.
- Existing `OwnedGemSourceConstruction::Unavailable` remains valid. Missing
  required fields do not default to unavailable or to a singleton.

The compiler derives the set from the entire validated `catalog.data().gems`
before assembling role rows. Reuse the existing
`SkillIdentityCatalog::gems_for_effect` query; it already retains all primary
Gem candidates without choosing a source iteration-order winner. Derivation
and offline reproduction use the same bounded routine. A subset of role rows
can still retain complete facts for its
own rows: removing another role must leave the surviving row's candidate list
unchanged, so a collision cannot disappear from its explicit observation.

The existing wire limit bounds input decoding. Runtime validation then charges
candidate count and text before building additional indexes or copied
collections, checks the canonical set and self membership, and rejects
contradictions with any included candidate row. It need not manufacture a role
for an omitted candidate.
The full reconstruction verifier remains the oracle for whether a caller's
claimed complete set actually matches the authenticated catalog.

This preserves the existing trust distinction: an adversary who rewrites both
the explicit candidate list and all commitments can create another internally
consistent claim, just as with today's effect-list observations; offline
reproduction must reject that forgery. Runtime digests are not signatures.
The new field eliminates the *inference* that a caller's role subset is complete;
it does not pretend to solve source authenticity through an extra Boolean.

## Consumer and bounded implementation

The generated-input adapter checks `Some([own_catalog_key])`, exact
game/variant/primary and mapped identities, plus the existing complete singleton
construction predicate. It retains every live provider/output/origin/preset
check. The role lookup is charged; no per-occurrence whole-catalog scan or new
runtime catalog index is needed.

Only the already successful `capture_checked` field sets gain exact present
secondary nil/false/true and group MAIN/CALCS nil/1 responsibilities. The
physical secondary and group-selector helpers keep their own source authority.
No generated source becomes a physical Gem, SkillUse or Action.

Level normalization and saved-object retention remain an independent boundary.
The source supplement tests singleton selection meaning on retained and newly
created groups. It does not assert equal objects, level, quality, count,
participation or native outputs. Those continue to need their existing separate
proofs and conjunctive inventory destinations. Archived generated rows emit no
checked token; inventory=None must retain identical work and output.

Compilation should charge the complete input-key census once, then the actual
copied candidate collections and text. Colliding groups can make repeated output
lists large; the existing aggregate/collection/wire limits must reject excess
before allocation. Do not silently deduplicate source Gem keys or cap candidates
to obtain a positive result.

## Alternative: require full reproduction at the loading boundary

Reuse `verify_owned_skill_catalog_reproduction` and make a verified role-index
state a prerequisite for generated reverse-routing claims. Mere construction of
`OwnedSkillRoleIndex` would not set that state. Loading would have to receive the
catalog, authenticate its commitment, reproduce it, and retain that fact for the
loaded artifact's lifetime.

This supplies stronger immediate omission detection without adding the new
serialized field. It also changes existing normalization/release entry points
or adds an import-loading dependency: the current catalog is 2,247,063 bytes on
disk before parsing, including declaration history and source observations.
Verification costs can be amortized once per immutable loaded catalog, but the
current CLI creates a new index for each command. No timing or memory multiplier
has been measured here.

The catalog would remain Import-only, not enter Core/Data/Engine's native
evaluation API. Still, this adds a runtime dependency on source-shaped export
data that the accepted build-time compilation layering aimed to avoid. A
serialized `verified: true` flag would not enforce this alternative and is not
recommended.

| Choice | Main benefit | Main cost |
| --- | --- | --- |
| Explicit compiler-derived candidate fact | Small, reusable runtime observation; removing a role cannot invent uniqueness; no runtime source catalog | Required artifact-field rebuild; offline reproduction remains the source-authenticity gate |
| Enforced catalog reproduction while loading | The consumer has an actual complete-catalog verification result | Runtime catalog input, parsing/reproduction cost and loading/API changes |
| Scan current role rows/count | No artifact change | Unsound omission inference; rejected |

## Invalidation and rebuild

Update the current unshipped shape in place; no compatibility reader or version
branch. Adding/removing a source catalog Gem changes the observations for its
primary family. Changed source effect identity or shadowed winning declarations
must be reproduced from the complete final catalog, not patched from old roles.

Rebuild current role artifacts and compiler transition, executable seeds,
role-bound normalization/Direct/generated/usage/source-action policies and their
tree/disposition dependencies through existing rebinding. Rebuild executable
fixtures containing Observed construction. Hand-authored unavailable facts stay
explicitly unavailable. Preserve all old raw/source evidence as historical data.

Native recipe/schema/rules/mapping identities and numerical coverage should be
byte-identical unless an independent change requires otherwise; this proposal
allocates no native definitions. The source catalog itself need not change.
Authenticate full artifact inverses and reimport all five unchanged originals.
This fact alone retires no issue.

## Required tests and next decision

1. Complete: the bounded omission regression confirms the exact
   structural/offline distinction.
2. Compiler and reproduction: one candidate, colliding singleton/multi-effect,
   same native target with different source primary, and shadowed declarations.
   Candidate lists derive from the full final catalog even if a later caller
   takes a role subset. Acquisition separately rejects aliased Skill objects
   and distinguishes additional-stat-set references as described above.
3. Runtime: missing required field, explicit unavailable, empty/duplicate/
   unsorted keys, missing self, present-row contradiction, namespace/mapping
   mismatch and nested/aggregate/text limits.
4. Omit the colliding role while retaining the target fact: generated receipt
   still refuses. Forge the target list to singleton and recompute its digest:
   offline full reconstruction rejects it.
5. Run the source-component tests, including real decoder/routing, reverse
   alias counterexample and retained/new-group selector equivalence. They remain
   source evidence, not native parity or Import authority. All eighteen tests
   now pass, including four additional constructed-selector controls; see
   `runs/generated-routing-and-selection-source-tests-02.log`.
6. Then implement the narrow private generated receipt and actual-producer,
   stale-provider, cross-preset, separate-consumer and atomic budget regressions.

Decision needed: accept the explicit compiler-derived candidate fact in the
existing Import artifact, or enforce full reproduction in the loading API.
Do not start the consumer by assuming the current role package is complete.

## Exact implementation seams and probe command

- `owned_skill_catalog/source_construction.rs`: add and bound the required
  observation, derive it from `gems_for_effect`, and reproduce exact contents.
- `owned_skill_catalog.rs`: validate included-row consistency while preserving
  runtime structural validation versus offline source reproduction.
- `owned_catalog_recipe.rs`: retain the existing full reconstruction call.
- `owned_normalize/generated_skill_inputs/dispositions.rs`: only after the
  artifact contract is accepted and rebuilt, consume the explicit singleton
  observation at successful live `capture_checked`.
- `owned_normalize/usage_inputs/inventory.rs`: transport those narrow field
  facts through the existing generated receipt; no parallel receipt model.

The bounded command for the maintained regression is:

```powershell
cargo test -p poe-optimizer-import --test owned_skill_catalog structurally_valid_role_subset_cannot_prove_reverse_primary_uniqueness -- --exact --nocapture
```

The draft intentionally asserts the current acceptance/rejection boundary; it
does not prescribe retaining subset acceptance after a future loading-contract
decision. Remove the temporary inclusion after recording the result.
