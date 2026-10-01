# Effective gem and skill inputs

Physical gem inputs, support preparation and final skill inputs are different data.
The evaluator must retain all three when a modifier changes a gem's level or quality.
The definition package supplies eligibility, contribution channels, units and level
domains. Build input supplies physical occurrences and raw values. Neither an import
adapter nor a user-facing client supplies a cached PoB effective level as a native fact.

## Input stages

The shared offline recipe emits ordinary owned rule programs for two phases:

| Phase | Exact output scope | Level calculation |
| --- | --- | --- |
| Active before supports | Authored Skill occurrence, Count quantity | Add the raw corruption delta, clamp to at least one, then add eligible external adjustments. Preserve the unvalidated result. |
| Support preparation | SupportOrigin assignment | Add eligible external adjustments to physical level, then validate against the injected dense domain. Corruption delta does not participate. |

Quality uses the selected physical quality plus eligible external quality adjustments.
The active corruption rule does not depend on the separate `corrupted` Boolean. Source
property eligibility uses physical gem metadata; support-added final skill types do not
retroactively change it. Global contribution channels are appropriate only for proven
global property families. Slot, socket, group and threshold filters require their exact
relations and predicates; they cannot become global additions.

The support recipe's dense level policy supplies the maximum and a valid natural fallback. Values outside
the range clamp first. An interior fractional level selects the natural fallback, rather
than being truncated into a valid level. Native arithmetic expresses this with ordinary
quantity operations, integer quantization and an exact round-trip comparison. Sparse
source tables need a separate policy; the source's arbitrary table-iteration fallback
must not become an invented ordering in the evaluator.

Active level validation belongs after supported-gem properties for an enabled skill.
Validating earlier would lose information: an intermediate fractional level can become
an integer when a later property applies. The disabled-skill branch is separate and
cannot establish enabled-skill ordering. A future final-input recipe reuses the reviewed
validation algorithm at its proper stage.

Quality absence is explicit policy. A rule may require a selected known quality and
leave absence unresolved. Selecting zero for absence additionally requires a complete
singleton quality-kind declaration. Partial membership, unknown quality and another
quality kind cannot silently become ordinary quality zero.

## Authoring and runtime boundary

`compile-owned-effective-gem-inputs POLICY --definitions SCHEMA --output NEW` compiles
an exact-schema-bound policy into `programs.json` and `report.json`. The command performs
no source execution, release installation or build calculation. The portable Import
library owns the bounded compiler; the CLI owns file reads and atomic publication.

The output contains rule fragments with exact owners and phases, not owner-completeness
declarations. A full release author explicitly installs them, retaining its existing
coverage gaps and binding every dependent artifact. IDs, units, coefficients, level
domains and selected channels are injected data. The runtime continues to execute the
existing typed native operations without source names, Lua callbacks or fixture dispatch.

Shared output stat IDs are safe across different physical gems because their values
belong to exact Skill or SupportOrigin occurrences. Final values must not be accumulated
on Player, where multiple gems would collide. A global contribution channel may be
reduced only when the ordinary contributor-completeness checks succeed. Empty-channel
zero is not a fallback for missing mechanics.

## Final inputs and generated skills

Active pre-support outputs are not final required parameters of a generated Skill.
After support admission, supported-gem properties can further change level and quality.
That step needs explicit physical source-gem membership and once-per-gem bookkeeping,
including shared or multiple effects and actor modifiers. Per-action support lists alone
do not prove that membership. Effects supplied by support gems have their own inheritance
rule and must not inherit the support gem's physical level by default.

Preparation reads independent authored-root inputs. Final consumers run after admission
and project into the exact supplied Skill's parameters. This separation must be checked
with a dependency witness: requiring final projected inputs to prepare their own support
selection would introduce a cycle. Removing activation gates or rewriting frozen prefix
values is not a valid way to resolve that cycle.

The historical Twister and Skeletal Sniper primary-supply programs project raw physical
values directly into final parameters. New endpoints must explicitly retire those
effects. A missing final producer remains unresolved until the replacement is complete;
pre-support outputs cannot be renamed to satisfy that requirement. Historical release
bytes and V1/V2 migration append guards remain unchanged. Full release assembly permits
an explicitly authored correction, with independent preservation tests for the exact
predecessor, registry, unrelated rules, coverage, source evidence and original queries.

## Validation and remaining work

Independent reference tests use authenticated, unchanged functions from the pinned PoB
checkout. Unchanged original builds establish real contributor provenance and input
ordering. Labelled component probes cover the numeric and role boundaries; they are
distinct from full-build parity. Native tests must exercise multiple owners and physical
occurrences, incomplete inputs, contributor closure, repeated scratch use and parallel
evaluation where applicable.

The supplied Cleric's physical level 19 becomes 30 through four observed global minion
gem-level item contributions of 4, 2, 2 and 3. The supplied Sniper's physical level 20
becomes 22 through two contributions of 1. These observations identify conversion work;
the totals do not belong in the evaluator as constants. Meat Shield II also matches the
physical minion-gem property, but validates back to its only supported level, 1. A separate
enabled-skill source probe confirms that physical level 12 with corruption delta 0.25 and
a later supported-gem level adjustment of 0.75 reaches 13, even when `corrupted` is false.

The remaining integration work includes actual modifier-family conversion, physical
gem placement for scoped properties, post-admission source-gem bookkeeping, final input
projection, actor-level production and complete import/declaration coverage. The
[implementation log](implementation.md) records tested delivery and the next executable
checkpoint. No component result alone establishes a complete native original build.
