# Exact-source action query targets

The owned normalizer can translate caller-authored action query correspondence into the
existing Core `ActionSelectionDraft`. It resolves already materialized physical SkillUse
occurrences; it does not choose a saved PoB view, create a skill, or prove activation.
See [normalization](owned-normalization.md) for the broader import boundary and the
[selected-actions data](../data/owned/poe2/3887ae68/selected-actions/README.md) for reviewed
real-build examples.

## Input contract

`ImportQueryTemplate` retains its caller-assigned query ID, external metric selector, and
explicit target. `ImportQueryTarget` supports `Player`, `Unresolved(code)`, and
`Action(ImportActionTarget)`. The older two variants retain their serialized shape.

| Type | Meaning |
|---|---|
| `ImportSkillUseLocator` | Exact `source_sha256`, source `occurrence_ordinal`, and `expected_gem: GemDefId`. |
| `ImportProviderTarget` | A skill-use locator plus an ordered, explicit list of declared grant slots. |
| `ImportActorTarget` | Player, or an independently located provider plus a declared actor slot. |
| `ImportActionTarget` | Action provider, actor, declared output, and explicit part, mode, and stat-set IDs. |

The source hash must equal `SourceProjectEvidence::identity().source_sha256`; use the hash
and ordinal from that evidence, not an import-string hash, UI index, display label, or owned
instance number. The hash is 64 lowercase hexadecimal characters. Every typed declaration
and definition must use the same namespace, matching the normalization package. The actor
provider and action provider each carry their own locator and path. Rust boxing of the
owned actor's provider adds no JSON wrapper.

An action target does not provide an actor-only query variant. Its owned actor is part of
the selected action's address. Its part, mode, and stat set are supplied data; the converter
never chooses the first declared member or interprets source `nil` as a selection.

## Resolution and provenance

For each locator, the converter checks the exact snapshot and looks up that source ordinal.
It requires an unnamespaced Gem occurrence with exactly one materialized Skill origin link.
That SkillUse must reference a physical Gem whose known owned definition equals
`expected_gem`. Equal Gem definitions in different source occurrences remain distinct.
Support-only or provider-generated source rows do not become physical SkillUses for a query.

The resulting provider root is the freshly allocated `ProviderRoot::SkillUse` for that
occurrence. Grant paths, actor slot, output, part, mode, and stat set are preserved exactly.
Reimporting the same source with a new lineage therefore targets the new owned instances
without copying old instance IDs. This correspondence does not certify that the supplied
paths are reachable or that an independently specified actor belongs to the action.

Only after all required locators resolve does the converter link their source occurrences
to the query preset. Shared origins and repeated query targets add one such link per source
occurrence. A failed target adds no partial successful-source link; its Pending issue is
recorded at the source root. Query IDs, order, and row count are preserved, including
unresolved metric or target rows.

Source hashes and ordinals stay in Import templates and evidence. Core receives owned
instance/definition references and existing draft types. The sidecar commits the ordered
query list together with the normalization policy using `owned-normalization-policy-v3`;
this differs from the package's policy-only normalization commitment. Full
[release assembly](owned-releases.md) separately commits the query artifacts. Changing
correspondence requires publishing the changed data and commitments explicitly.

## Failure and coverage boundaries

| Situation | Result |
|---|---|
| Well-formed hash names a different snapshot; ordinal is missing or identifies the wrong record | Pending target with an explicit source diagnostic. |
| No unique materialized SkillUse, unresolved Gem, or different expected Gem | Pending target; no first-match or player fallback. |
| Action locator succeeds but owned-actor locator fails | Entire action target remains Pending. |
| Malformed hash, mixed namespace, wrong typed wire tag, unknown field, or duplicate query ID | Input/policy error. |
| Path, work, bytes, collection, or origin-link limit exceeded | Bounded operation fails; no truncated successful target list. |
| Source correspondence resolves but topology/actor/output compatibility is unknown or wrong | The address is retained; subsequent Core binding diagnoses unresolved, unavailable, or invalid topology. |

A known action address is not a bound evaluation request. Core still checks exact provider
reachability, actor ownership, output exposure, and part/mode/stat-set membership. Engine
separately checks supply, activation, required inputs, rule coverage, and numerical results.
Metric identity mapping remains independent and can stay Pending even when its target is
known. Disabled skills and inactive grants are not activated by selecting their outputs.

The converter neither finalizes incomplete drafts nor closes parameter, choice, or rule
collections. In particular, the current minion output-port topology does not supply an
actor ability. The separate [actor skill supply proposal](owned-actor-skill-supply.md)
describes that unresolved contract; correspondence must not silently redirect an old output
selector to a future generated ability path.

## Implementation and checks

The [resolver](../crates/poe-optimizer-import/src/owned_normalize/query_targets.rs) builds
bounded SkillUse/Gem indexes once when action queries are present. Validation charges
aggregate target work and bounds each provider path before cloning; resolution also charges
origin scans, lookups, and links. The existing normalizer's byte and draft limits still
apply. No PoB runtime, source callback, legacy skill profile, or evaluator is invoked.

The Rust [contract tests](../crates/poe-optimizer-import/tests/owned_query_targets.rs) cover
repeated equal Gem definitions, fresh import lineages, independent actor/action roots and
paths, stale actor correspondence, unmatched/unmaterialized origins, ambiguous mapping,
invalid namespaces/hashes, tightened bounds, and old/new wire behavior. Fixture-specific
selection evidence belongs in the linked data directory. New tooling and tests use Rust;
existing Python utilities are unchanged.
