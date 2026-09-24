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

## Numeric-alias successor

`numeric-alias-normalization.json` is the explicit complete successor normalization
policy for the package above. It preserves every previous recipe and binding, changes
the policy version to `physical-support-numeric-aliases-v1`, and adds the finite alias
`{"token":"nil","replacement":"0"}` to exactly the 514 Quantity corruption-delta
recipes. The original `policy.json` remains the reproducible predecessor.

The replacement goes through the same Scientific Count codec and scale as ordinary
numeric text. Boolean flags remain independent. Missing/unavailable inputs and other
malformed spellings remain unresolved. The rule is injected configuration, not an
engine default or a build-name exception. The full-policy artifact intentionally binds
the exact predecessor schema; reauthor and validate those bindings for another package.

```text
poe-optimizer publish-owned-normalization runs/owned-support-gem-inputs-01/package --normalization data/owned/poe2/3887ae68/support-gem-inputs/numeric-alias-normalization.json --output runs/owned-numeric-aliases-01/package
```

This normalization-only publication preserves the schema, registry, rules, routes,
item policies and query identities. It rebinds unchanged tree content to the explicit
new normalization identity. No Gem membership is closed by adding a known scalar.
Fresh normalization of the five originals adds 159 Quantity values (0/0/47/46/66), all
zero, and retains the original source text. Both Boolean and Quantity input totals become
337. The 478 physical Gems, 110 queries and 12 Complete / 466 Pending parameter collections
are preserved. Living Lightning II remains outside the single-effect family. All five
builds still report calculation not run; these input conversions are not build parity.
