# Gem support-source domains

This packet supplies the Gem-definition part of composed support discovery.
The existing recipe extension contains **966 explicit declarations: 818 Known
and 148 Unmapped**. It adds no evaluator code, programs, definitions, tables or
runtime source-language fields. Item, modifier, passive, Actor, Skill and other
selected owners retain their separate coverage requirements.

## Evidence and boundary

The complete authenticated catalog in `../empty-payload-inventory/source-records.json`
records primary, declared additional and actually constructed effects, including
their support classification. Its bindings map exact source identities to owned
Gem definitions. Publication authenticates the retained independent JIT reports,
rechecks every binding against the current mapping, and pins the source consumers.
It does not start a new PoB process or use observed empty modifier buckets as
absence evidence.

At the pinned revision, `Data.lua` constructs primary and additional Gem effect
lists before build calculations. `CalcSetup.lua:630` processes an effect as a
support only when its definition has that classification. Lines 1966–1969 call
that function for the primary and each constructed additional effect. Raw Gem
level/quality and display order do not create another support origin. A primary
support is already represented by an authored assignment. Every declared
additional reference must resolve and agree with the complete constructed list
before this packet certifies that list contains no additional support.

The result is deliberately independent of whether an effect is a trigger,
whether a support is enabled or eligible, and which action is queried. Numerical
effects and descendants remain subject to their own native coverage. The
item-derived `ExtraSupport`, cross-slot `LinkedSupport` and source-group sharing
paths have different providers and are **not** covered by this Gem declaration.
Neither is the exact tree-provider `noSupports` exclusion.

Twenty-nine Gems have actual additional support effects and keep
`additional-gem-support-origins-not-converted`. Another 119 have unresolved
declared effect construction and keep `gem-effect-construction-unresolved`.
In particular, the pinned source declares `CommandSkeletalSniperPlayer` but does
not resolve or construct it. That is not enough evidence to classify the intended
game mechanic as absent. The packet copies neither missing-table behavior nor a
default false support flag into the native model. Review these source discrepancies
before refining the affected domains; adding a future definition must never inherit
an old absence claim.

The selected Original05 physical Gem occurrences now have 18 Known domains and
five Unmapped domains: Skeletal Arsonist, Skeletal Sniper, Skeletal Frost Mage,
Skeletal Reaver and Ice Nova all have unresolved declared references. Manual and
tree-generated Djinn are independent Skill/provider owners, not additional
physical Gems. None of these counts certifies complete native discovery.

## Validation and reproduction

`owned_gem_support_domains` validates the entire catalog and tests extra support,
missing/malformed evidence, unresolved references, duplicate effects and display
ordering. Its publication test uses the shared release assembler and preservation
checks. The whole-input inverse permits only the explicit domain declarations
and their provenance receipt; schema, rule bodies, inputs, selections and queries
are unchanged. Rule and compiled identities change.

Set `POE_OPTIMIZER_TEST_GEM_SUPPORT_DOMAINS_PRIOR` to the checked predecessor and
`POE_OPTIMIZER_TEST_GEM_SUPPORT_DOMAINS_OUTPUT` to a fresh directory, then run:

```text
cargo test --locked -p poe-optimizer-cli --no-default-features --test owned_gem_support_domains publish_gem_domains_preserving_all_five_originals -- --ignored --exact
```

Publication `runs/owned-gem-support-domains-publication-01` passed in 20.98s,
including byte-identical rebuilding and fresh before/after imports of all five
originals. Selected input counts remain 107/117/109/123/4, with all 110 queries.
The source catalog/proof files stay in offline authoring; only owned declarations
enter `rules.json`. Complete native builds remain **0/5**.

All 76 joined Sniper checks pass against the successor, all seven common native
discovery-gate checks pass, and strict Clippy passes for the new integration target.
These retain disabled/provider-change/refusal and fresh/reused/Rayon controls.
The native source gate still prevents these partial game domains from certifying
a complete build.
