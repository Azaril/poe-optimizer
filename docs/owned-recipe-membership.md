# Compact recipe membership authoring

The offline Import compiler accepts finite membership additions without requiring callers
to restate full existing descriptors. Published evaluator packages still contain ordinary
materialized owned schemas, rules and routing. No patch interpreter or template selector
is needed during evaluation.

Use `extend-owned-recipe PRIOR --extension EXTENSION --membership-patch PATCH --output NEW`.
The extension remains the existing version-1 allocation/rule format. Omitting the optional
patch preserves the existing command, receipt and digest contracts. Explicit successor
item policies still require paired `--items` and `--item-source` arguments.

## Contract

A version-1 patch names exact prior registry, schema, rules and routing identities, plus
the digest of the separately supplied extension. Each `item_template_modifiers` group
contains sorted, unique, typed template and modifier IDs. Targets are finite: no wildcard,
name matching, source lookup, all-current-items selector or future default is allowed.

Only Known templates whose modifier membership is Partial can gain members. Every
target can appear once; every added modifier must resolve to a Known modifier in the
prior or supplied extension. Unknown, foreign, Unmapped, Complete, duplicate and already
present members are errors. A patch cannot also target a full descriptor in the extension.
It cannot modify scalar fields, other membership sets, closures or coverage gaps.

The compiler expands additions into ordinary V1 records and sends them through the
existing V3 membership refinement and successor checks. Replaying the same inputs gives
the same materialized schema, rules and proof as an explicit V1 extension. Compact syntax
is an authoring convenience, not authority to infer item legality or close coverage.

## Bounds and evidence

Decode bounds the request before deserialization. Compilation separately bounds groups,
target/modifier references, their insertion product, serialized bytes and work. Borrowed
descriptor preflight counts exact expanded wire size before cloning; hashing, serialization,
copying and membership comparisons consume work. The existing 16 MiB expanded-extension
and four-million extension-work caps remain in force, alongside the patch compiler's own
bounded budget and all inner recipe limits.

The receipt binds the patch, supplied and expanded extensions, both schema endpoints and
prior component identities. It reports patched templates, inserted members, expanded
bytes and work. Publication keeps its no-overwrite and checked predecessor behavior.

The initial consumer is the four ordinary item attribute families. Their finite membership
patch names all 1,756 reviewed base templates once. This does not establish affix eligibility,
activation or complete item/build evaluation. Rust tests compare explicit/compact outputs,
exercise stale endpoints, invalid targets and exact resource boundaries, and run the native
CLI chain with original-build invariants.