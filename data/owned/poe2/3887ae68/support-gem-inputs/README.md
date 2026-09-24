# Physical support Gem input conversion

`policy.json` is explicit offline configuration for 514 physical support Gem identities
in the pinned skill identity catalog. Selection covers every support with a single
resolved primary effect and no declared/constructed additional effects or stat sets.
The constructed singleton Concussive Runes declares an unresolved additional effect;
it is excluded. Selection is by exact source identity, not by the supplied character builds.

The Rust compiler joins the catalog through the checked mapping and role indexes. It
rejects stale catalog/source bindings, ambiguous external variants, provider-only entries,
Known-schema rewrites and unreviewed potential effects. Stable owned Gem IDs are retained;
two owner-scoped parameter slots are allocated for each newly reviewed Gem.

The injected schema admits level 1 and optional standard quality. The existing quality
amount envelope is unchanged; it is not a claim that the source loader rejects other
quality values. Both parameter and all other Gem memberships remain Partial. This is
input knowledge, not support activation, rule coverage or complete native evaluation.

The two recipes retain independent inputs:

- `corrupted`: exact `true`, `false` and `nil` tokens become a Boolean.
- `corruptLevel`: signed finite scientific/decimal syntax becomes a Count quantity,
  preserving fractional values. The envelope covers finite binary64 values. Missing,
  literal `nil`, malformed and non-finite numbers remain unresolved.

In the five original builds these recipes preserve 337 Boolean flags and 178 numeric
deltas; another 159 selected literal `nil` deltas remain unresolved. The previous 12
Complete / 466 Pending parameter collections and all 110 query rows are unchanged.
This does not establish a complete native build evaluation.

There is no Lua conversion fallback. Missing inputs remain Pending. The full reference
loader accepts some spellings outside these finite recipes; those are not silently
normalized into invented values. No source field name enters the evaluator.

## Reproduce the owned package

The commands are source-free and run in the native CLI. Each output directory must be
new. Compilation prepares both transitions and validates them before publishing its
three artifacts; the following commands independently perform the checked publications.
The intermediate schema package remains available to audit the actual predecessor.

```text
poe-optimizer compile-owned-gem-inputs runs/owned-gem-inputs-01/package --catalog data/owned/poe2/3887ae68/import/skill-identities.json --policy data/owned/poe2/3887ae68/support-gem-inputs/policy.json --output runs/owned-support-gem-inputs-01/compiled
poe-optimizer migrate-owned-gem-schemas runs/owned-gem-inputs-01/package --migration runs/owned-support-gem-inputs-01/compiled/migration.json --output runs/owned-support-gem-inputs-01/schema
poe-optimizer publish-owned-normalization runs/owned-support-gem-inputs-01/schema --normalization runs/owned-support-gem-inputs-01/compiled/normalization.json --output runs/owned-support-gem-inputs-01/package
```

The two prior neutral Gem recipes remain in the generated normalization policy. Their
reviewed closure does not extend to the new support Gems. Every prior program, route,
item policy and query is preserved apart from required schema binding changes.

The optional Rust reference test `owned_physical_gem_inputs` validates selected identities,
complete source loading, level and scalar facts in both JIT modes. Native compiler tests
cover binding, topology and input-policy rejection. CLI tests exercise the complete
publication chain and normalize all five untouched originals without a PoB checkout.
See [the Gem contract](../../../../../docs/owned-gem-inputs.md) and
[the implementation checkpoint](../../../../../docs/implementation.md) for measured coverage.
