# Gem support-source domains

This packet supplies the Gem-definition part of composed support discovery.
The existing recipe extension contains **966 explicit declarations: 937 Known
and 29 Unmapped**. It adds no evaluator code, programs, definitions, tables or
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
effect reference, including setup-generated fields, needs a checked support
classification. Loaded effects must agree with the constructed list. The eight
missing runtime effects use the separate export evidence described below;
their missing construction and mechanics remain unresolved.
Stat-set references are checked as metadata, not resolved as effect origins.
Unknown reference kinds, missing rows or disagreement remain blocking.

The result is deliberately independent of whether an effect is a trigger,
whether a support is enabled or eligible, and which action is queried. Numerical
effects and descendants remain subject to their own native coverage. The
item-derived `ExtraSupport`, cross-slot `LinkedSupport` and source-group sharing
paths have different providers and are **not** covered by this Gem declaration.
Neither is the exact tree-provider `noSupports` exclusion.

Twenty-nine Gems have actual additional support effects and keep
`additional-gem-support-origins-not-converted`. The eight missing runtime effects
are now independently classified as active effects, closing only their Gem
support-source gaps. No missing skill is materialized, aliased to another skill
or considered numerically implemented.

All **23 selected Original05 physical Gem occurrences** now have Known domains,
including Skeletal Arsonist, Sniper, Frost Mage and Reaver. Manual and
tree-generated Djinn are independent Skill/provider owners, not additional
physical Gems. None of these counts certifies complete native discovery.

The initial publication incorrectly classified all 111 Gems with additional
stat-set metadata as unresolved effect construction, including Ice Nova. This
was our offline proof's defect, not a PoB defect or missing game mechanic. The
current packet corrects that classification using the already available typed
catalog. It rebuilds from the same predecessor without a compatibility path or
weakening the extension API's protection against overwriting prior authority.

## Missing runtime effects: reviewed export classification

`export-classifications.json` retains nine exact blocks for eight missing effect
IDs from pinned `src/Export/Skills/SkillGemsExport.txt`; Sniper appears twice.
`skillGemList.lua:13–34` emits `#flags` only for a non-support granted effect.
This is the effect's `GrantedEffects.IsSupport`, not the containing Gem's class
or an ID/name convention. All eight reviewed blocks contain that active branch.
Three resolved controls include an additional active effect and two support
effects; their classifications agree with the typed runtime catalog. Concussive
Runes is a useful contrast: its primary is a support, its missing additional
effect is active. The proof would retain a positive additional-support gap if
an independently classified missing effect were a support.

The generated template has SHA-256
`5aa4cf3055ed80696c227e8fda28332afddc935ee9f6dc31caedede9833c73f2`
and Git blob `11782d377e29b384ac5b1f8669450d9195295e8f` at the pinned revision.
This `.txt` file is outside the Lua manifest. Publication reads its exact Git
object, verifies bytes/hash and every duplicate occurrence, and authenticates
the generator through the existing manifest. No new extractor, Lua interpreter,
runtime fallback, source-manifest revision or compatibility format is introduced.

The actual export templates loaded by `skills.lua` omit these Command records;
`SkillGemsExport.txt` is a generated inventory, not a loaded skill module. The
four selected families have distinct minion-side actions already represented
in `Minions.lua`: Gas Arrow, Explosive Demise, Ice Armour and Enrage. Those are
not aliases for the missing Player Command definitions. Costs, command usage,
recipient/target selection and effects need their own native ownership/data.
No parity exception or absence of game mechanics is asserted here.

Four bounded PoE2DB lookups on October 8 corroborate the distinction between
player Command and minion behavior: [Sniper](https://poe2db.tw/us/Skeletal_Sniper),
[Arsonist](https://poe2db.tw/us/Skeletal_Arsonist),
[Frost Mage](https://poe2db.tw/us/Skeletal_Frost_Mage) and
[Reaver](https://poe2db.tw/us/Skeletal_Reaver). They describe Command costs and
minion actions, but their current data is not pinned-version numerical evidence.
No tables or values from these pages were imported. Source export classification
alone supplies this packet's narrow authority.

## Validation and reproduction

`owned_gem_support_domains` validates the entire catalog and tests extra support,
missing/malformed evidence, unresolved references, duplicate effects, typed
reference categories across all 111 affected Gems, export classification corruption,
missing proof, duplicate occurrences and display ordering. Its publication test uses the shared release assembler and preservation
checks. The whole-input inverse permits only the explicit domain declarations
and their provenance receipt; schema, rule bodies, inputs, selections and queries
are unchanged. Rule and compiled identities change.

Set `POE_OPTIMIZER_TEST_GEM_SUPPORT_DOMAINS_PRIOR` to the checked predecessor and
`POE_OPTIMIZER_TEST_GEM_SUPPORT_DOMAINS_OUTPUT` to a fresh directory, then run:

```text
cargo test --locked -p poe-optimizer-cli --no-default-features --test owned_gem_support_domains publish_gem_domains_preserving_all_five_originals -- --ignored --exact
```

Publication `runs/owned-gem-support-domains-publication-03` replaces the prior
development endpoint and passes in 21.08s, including byte-identical rebuilding
and fresh before/after imports of all five
originals. Selected input counts remain 107/117/109/123/4, with all 110 queries.
The source catalog/proof files stay in offline authoring; only owned declarations
enter `rules.json`. Complete native builds remain **0/5**.

All 76 joined Sniper checks pass against the current successor in 37.83s, and
strict Clippy passes for the integration target. The seven common native
discovery-gate checks passed at the preceding checkpoint; runtime code is unchanged.
These retain disabled/provider-change/refusal and fresh/reused/Rayon controls.
The native source gate still prevents these partial game domains from certifying
a complete build.
