# Gem support-source domains

This packet supplies the Gem-definition part of composed support discovery.
The maintained release contains **966 explicit declarations: 937 Known
and 29 Unmapped**. It adds no evaluator code, programs, definitions, tables or
runtime source-language fields. Item, modifier, passive, Actor, Skill and other
selected owners retain their separate coverage requirements.

## Evidence and boundary

The complete authenticated catalog in `../empty-payload-inventory/source-records.json`
records primary, declared additional and actually constructed effects, including
their support classification. The existing typed `../import/skill-identities.json`
catalog distinguishes additional effects from stat-set metadata; both sources
must agree on every reference, identity and support classification. Bindings map
exact source identities to owned Gem definitions. The maintained Authoring
release tests authenticate the retained source evidence, recheck every binding
against the current mapping, and pin the source consumers. They do not start a
new PoB process or use observed empty modifier buckets as
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
and historical Git blob `11782d377e29b384ac5b1f8669450d9195295e8f` at the pinned revision.
This `.txt` file is outside the Lua manifest. Maintained tests authenticate its
normalized source bytes with SHA-256, verify every duplicate occurrence, and authenticate
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

The maintained `poe-optimizer-authoring` release target validates the complete
catalog directly against the release built from `migration-source/release.kdl`.
Its seven `gem_support_domains` checks retain the additional-support and
missing-evidence refusals, all 111 stat-set distinctions, eight reviewed missing
runtime effects, malformed categories, duplicate effects and export occurrences,
and display-order independence. Every source binding must match the current
release mapping; all 966 current Gem declarations must match the independently
derived source domains exactly. Other provider kinds retain separate evidence
and are not classified by this test.

```text
cargo test --locked -p poe-optimizer-authoring --test release gem_support_domains
```

These offline evidence tests require the pinned PoB submodule checkout. They
read normalized source bytes and the generated export template, check their
reviewed SHA-256 commitments, and verify every retained block occurrence. They
run no Git subprocess, Lua host or PoB evaluator. The normal maintained release
build itself does not read the submodule. The old ignored publication command
and its predecessor-stage helper have been removed; no local `runs/` artifact is
required. The original JIT reports remain historical validation evidence, not a
portable test prerequisite or a new parity claim.

`extension.json` and its authoring receipt remain immutable source evidence for
the migrated declarations; they are not a second publication path. Current
release construction, full original-build import checks and retirement of the
remaining historical helpers are covered by the maintained release target and
implementation plan. Complete native builds remain **0/5**.
