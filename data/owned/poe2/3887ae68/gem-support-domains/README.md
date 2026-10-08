# Gem support-source domains

This packet supplies the Gem-definition part of composed support discovery.
The existing recipe extension contains **966 explicit declarations: 929 Known
and 37 Unmapped**. It adds no evaluator code, programs, definitions, tables or
runtime source-language fields. Item, modifier, passive, Actor, Skill and other
selected owners retain their separate coverage requirements.

## Evidence and boundary

The complete authenticated catalog in `../empty-payload-inventory/source-records.json`
records primary, declared additional and actually constructed effects, including
their support classification. The existing typed `../import/skill-identities.json`
catalog distinguishes additional effects from stat-set metadata; both sources
must agree on every reference, identity and support classification. Bindings map
exact source identities to owned Gem definitions. Publication authenticates the retained independent JIT reports,
rechecks every binding against the current mapping, and pins the source consumers.
It does not start a new PoB process or use observed empty modifier buckets as
absence evidence.

At the pinned revision, `Data.lua` constructs primary and additional Gem effect
lists before build calculations. `CalcSetup.lua:630` processes an effect as a
support only when its definition has that classification. Lines 1966–1969 call
that function for the primary and each constructed additional effect. Raw Gem
level/quality and display order do not create another support origin. A primary
support is already represented by an authored assignment. Every additional
effect reference, including setup-generated fields, must resolve and agree with
the complete constructed list before this packet certifies that list contains
no additional support.
Stat-set references are checked as metadata, not resolved as effect origins.
Unknown reference kinds, missing rows or disagreement remain blocking.

The result is deliberately independent of whether an effect is a trigger,
whether a support is enabled or eligible, and which action is queried. Numerical
effects and descendants remain subject to their own native coverage. The
item-derived `ExtraSupport`, cross-slot `LinkedSupport` and source-group sharing
paths have different providers and are **not** covered by this Gem declaration.
Neither is the exact tree-provider `noSupports` exclusion.

Twenty-nine Gems have actual additional support effects and keep
`additional-gem-support-origins-not-converted`. Another eight have unresolved
declared effect construction and keep `gem-effect-construction-unresolved`.
In particular, the pinned source declares `CommandSkeletalSniperPlayer` but does
not resolve or construct it. That is not enough evidence to classify the intended
game mechanic as absent. The packet copies neither missing-table behavior nor a
default false support flag into the native model. Review these source discrepancies
before refining the affected domains; adding a future definition must never inherit
an old absence claim.

The selected Original05 physical Gem occurrences now have 19 Known domains and
four Unmapped domains: Skeletal Arsonist, Skeletal Sniper, Skeletal Frost Mage
and Skeletal Reaver have unresolved declared additional effects. Manual and
tree-generated Djinn are independent Skill/provider owners, not additional
physical Gems. None of these counts certifies complete native discovery.

The initial publication incorrectly classified all 111 Gems with additional
stat-set metadata as unresolved effect construction, including Ice Nova. This
was our offline proof's defect, not a PoB defect or missing game mechanic. The
current packet corrects that classification using the already available typed
catalog. It rebuilds from the same predecessor without a compatibility path or
weakening the extension API's protection against overwriting prior authority.

## Validation and reproduction

`owned_gem_support_domains` validates the entire catalog and tests extra support,
missing/malformed evidence, unresolved references, duplicate effects, typed
reference categories across all 111 affected Gems, and display
ordering. Its publication test uses the shared release assembler and preservation
checks. The whole-input inverse permits only the explicit domain declarations
and their provenance receipt; schema, rule bodies, inputs, selections and queries
are unchanged. Rule and compiled identities change.

Set `POE_OPTIMIZER_TEST_GEM_SUPPORT_DOMAINS_PRIOR` to the checked predecessor and
`POE_OPTIMIZER_TEST_GEM_SUPPORT_DOMAINS_OUTPUT` to a fresh directory, then run:

```text
cargo test --locked -p poe-optimizer-cli --no-default-features --test owned_gem_support_domains publish_gem_domains_preserving_all_five_originals -- --ignored --exact
```

Publication `runs/owned-gem-support-domains-publication-02` replaces the initial
development endpoint and passes in 21.07s, including byte-identical rebuilding
and fresh before/after imports of all five
originals. Selected input counts remain 107/117/109/123/4, with all 110 queries.
The source catalog/proof files stay in offline authoring; only owned declarations
enter `rules.json`. Complete native builds remain **0/5**.

All 76 joined Sniper checks pass against the corrected successor in 38.77s, and
strict Clippy passes for the integration target. The seven common native
discovery-gate checks passed at the preceding checkpoint; runtime code is unchanged.
These retain disabled/provider-change/refusal and fresh/reused/Rayon controls.
The native source gate still prevents these partial game domains from certifying
a complete build.
