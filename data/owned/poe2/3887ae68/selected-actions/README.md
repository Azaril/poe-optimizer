# Explicit selected-action query correspondence

The two complete query lists preserve the original 22 IDs, order and metric selectors.
Only four targets change: original 2 `reference-13`/`reference-15` select the existing Twister
player action, and original 5 `reference-14`/`reference-16` select the existing Skeletal
Sniper owned actor's Basic Attack output. The remaining three original lists stay unchanged.
All 110 measurements remain in the combined package.

These are injected Import templates, not special cases in normalization or evaluation.
Each skill-use locator commits exact source bytes, an occurrence ordinal and the expected
owned Gem identity. The normalizer joins that occurrence to its materialized SkillUse;
new import lineages produce their own instance IDs. A stale snapshot, a different Gem or
an occurrence without one unique authored skill remains Pending. No display-name or first
matching-Gem fallback is permitted. Other builds can provide their own locators and typed
actor/provider/output paths through the same API.

`selection-evidence.json` records the reviewed original selections. Original 2 uses skill
set 6, group 8, Gem 1 (Twister level 19 / quality 20), source occurrence 382. Original 5
uses skill set 4, group 3, Gem 1 (Sniper level 20 / quality 0), occurrence 211; its saved
minion is `RaisedSkeletonSniper`, action 1 / `MinionMeleeBow`. Those source labels are
correspondence evidence only. The production target uses owned IDs and explicit paths.

Actor and action providers are independent. Twister uses the player actor and its primary
grant path. Sniper's actor uses the primary provider plus population slot; its output
provider additionally traverses the population grant. The ordinary part, mode and primary
stat set are explicit reviewed selections. The literal source `statSetIndex="nil"` is not
an implicit selection rule.

These output selectors do not supply or activate a minion ability, finalize an incomplete
build, or resolve missing metrics. The existing actor ability activation gap remains. The
[actor-owned supply proposal](../../../../../docs/owned-actor-skill-supply.md) requires owner
review; a later accepted data release must explicitly publish any changed ability path.
Old selectors must not be silently redirected.
