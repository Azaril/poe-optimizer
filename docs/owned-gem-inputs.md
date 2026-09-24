# Intrinsic gem inputs during import

A Gem schema declares owned slots; it does not prove how an external document supplies
those slots. Even a Complete empty declaration requires an explicit, schema-bound
`GemInputPolicy` before the import adapter can close `GemDraft.parameters`.

Each injected rule selects an exact Gem definition, optional finite source-attribute
guards and typed parameter recipes. Guards distinguish missing, empty and exact text;
missing is never a wildcard. Recipes reuse the existing Boolean, Integer, Quantity and
Option codecs and explicit missing-value policy. There is no built-in corruption name,
Lua conversion fallback, skill-name dispatch or source operation in the evaluator.

Validation requires a Known Gem, slots declared by that exact Gem, compatible parameter
sites, exact units/types, valid explicit defaults and bounded policy/source work. A rule
closes a collection only if all source guards pass, the declaration is Complete, every
declared slot is covered and every required value converts. Known values survive under
Partial declarations. Unmapped owners, missing rules, failed guards, absent required
values and malformed/out-of-range values retain Pending coverage. Optional absence is
admitted only through an explicit recipe and OptionalOnce slot.

Policies that omit `gem_inputs` retain their serialized bytes and policy-v3 identity.
Fresh normalization now leaves their Gem parameter collections Pending; old persisted
drafts are not rewritten. Sidecar schema/domain **12** identifies the changed import
behavior. Consumers must not compare a v11 inference with a v12 conversion as an
unchanged-behavior replay.

The [reviewed neutral policy](../data/owned/poe2/3887ae68/gem-inputs/README.md) supplies
exact guards for the two existing Known Gem definitions. It restores the twelve original
empty parameter collections only for corruption text `false`/`nil` and delta text `0`.
Edited/nonzero, missing and unreviewed spellings remain Pending under that policy. This
is a deliberately finite admission policy, not the whole source loader's accepted language.

Level, quality, support target, use scope, choices, generated actors/actions, active
contributors and numerical coverage retain their separate obligations. The full source
LoadSkill audit also finds count, global activation switches, choices and per-effect
maps. Knowing the two corruption scalars does not prove an exhaustive input model.

## Physical Gem knowledge migration

Unmapped-to-Known Gem conversion uses a separate import-only V4 refinement; the existing
V3 Known-schema membership contract is unchanged. The exact prior schema, registry,
source pin, mapping and role evidence bind the migration. Only listed physical Gems
with a known role and primary skill may be promoted. Provider-only identities are rejected.
New registry entries may only be explicitly declared parameter slots owned by those Gems.

Every previous definition, slot, program body, closure and route remains unchanged except
for the listed Gem knowledge and necessary artifact identity rebinding. Potential skill
membership and unreviewed choices, grants, actors, outputs and sockets remain Partial.
The currently reviewed two-scalar conversion must also retain Partial parameter membership
until the remaining saved inputs have an owned classification. Registry/source evidence
is not numerical game-rule coverage.

`migrate-owned-gem-schemas` hosts the reusable checked migration. A separate
`publish-owned-normalization` operation installs a complete caller-supplied policy,
validates the true previous policy/tree, preserves recipes and all query rows, and binds
unchanged tree content to the new policy. Stale authored bindings reject; they are never
silently repaired. Both commands publish to a new directory only.

Normalization allocates fresh occurrence and issue identities. Resolving an issue may
shift later allocator-local IDs. Compare independent runs through source attribution and
semantic values while preserving query identities/order; do not allocate phantom issues.

Support receiving and activation remain the separately proposed
[support contract](owned-support-activation.md). These input APIs introduce no new Core
or evaluator contract and do not complete any original build.

## Catalog conversion

`compile-owned-gem-inputs` is a source-free offline compiler for an explicitly selected
physical Gem family. The caller supplies a finite identity catalog and reviewed schema,
quality membership, parameter types and lexical recipes. No Gem name, level domain,
corruption field or default is embedded in the native calculation engine.

The compiler binds the catalog to the original role-compilation digest and immutable
source-file pins. Every selected source row needs an exact unique external mapping,
matching primary effect, known physical role and an Unmapped prior Gem descriptor.
It allocates owner-scoped slots in canonical Gem order and retains Partial membership
for every generated Gem collection. A shared policy is expanded only within bounded
input/output resources. Existing known definitions and rules are preserved.

Compilation emits the explicit V4 migration, successor-bound normalization policy and
both checked transition receipts. Separate publications keep the true previous policy
and the intermediate schema package auditable; neither silently repairs stale bindings.

The [reviewed support family](../data/owned/poe2/3887ae68/support-gem-inputs/README.md)
contains 514 single-declared-effect identities and two independent typed inputs. Fractional
corruption deltas use a Quantity rather than an Integer. Source-loader acceptance outside
the injected lexical/domain policy remains Pending. These declarations supply known
inputs for later support resolution; they do not implement support delivery or calculation.
