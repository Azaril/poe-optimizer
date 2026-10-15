# Proposal: numeric progression through existing build rewards

**Status:** Proposed; no numeric reward adapter implemented.
**Date:** 2026-10-15.
**Decider:** Project owner.

## Actual ownership

The source control `configBossFaceBroken` counts unique campaign bosses defeated.
It is not a current Enemy condition, item roll or temporary combat state.
`ConfigOptions.lua:860–862` describes that meaning, and
`ImportTab.lua:833–846` obtains the value from character quest statistics and
writes the configuration Placeholder. A Player modifier-list destination is
transport, not authority to reclassify progression as an encounter assumption.

The domain design already assigns progression/rewards to `BuildSpec`.
`CharacterSpec` has class, ascendancy, level and rewards; it has no general
parameter bag. Existing `RewardSelection` has typed parameters, and selected
Character and Choice rewards compose into the concrete Character's rewards.
Config-derived quest selections already use this Choice reward path.

Original05 saves Placeholder 56. There is **no constructor default** for this
control. Neither 56 nor zero may be used as an absent-value fallback. Present
zero, no selected candidate, malformed input and an unresolved value require
distinct evidence; a skipped callback is not an authenticated zero count.

## Recommendation

Use a data-defined **progression Reward with a typed numeric parameter**, selected
through the existing **build Choice preset** associated with the imported
configuration scope. This keeps the value on the Build side and preserves its
saved alternatives without copying it across independent Character presets.
The term Reward here denotes the existing progression/effect source contract;
the counter does not imply an extra quest reward or a new item bonus by itself.

Extend the current `OwnedRewardPolicy` to project reviewed numeric source values
into declared Reward parameters. Its current Boolean/Option adapters use fixed
parameter templates and cannot transport this captured number. Reuse checked
source selection, exact parameter authority, reward provenance and configuration
accounting; do not create another progression catalogue or Player parameter owner.

Retain the existing Reward-to-Player contribution authority when an actual
mechanic consumes the count. Source projection alone supplies no damage rule,
supplier-completeness certificate or item-specific multiplier. Keep names and
PoB control behavior at Import; native rules consume the owned progression fact.

| Option | Benefit | Cost |
| --- | --- | --- |
| Keep imported progression in existing Choice rewards | Preserves ConfigSet alternatives and current independent-axis composition; needs only a bounded numeric reward adapter. | The source's authored progression remains a selected build choice, rather than automatically updating the Character preset. |
| Reconcile directly into Character rewards | Places the fact directly on its natural persisted character owner. | Needs explicit conflict handling across ConfigSets/Character presets; selecting a value or duplicating it across characters would lose source intent. |

Recommend Choice rewards for source-authored alternatives. A later actual
character import may populate Character rewards through its own authenticated
adapter. If both sources provide the same progression fact, existing additive
reward composition is not enough: reject conflicting or duplicate authority
until a reviewed reconciliation contract exists. Do not sum two campaign counts.

This check does not exist merely because rewards compose today. Current
composition deduplicates identical `RewardSelectionId` references, but accepts
distinct reward occurrences with the same definition. Keep one shared occurrence
referenced from both presets once. For distinct occurrences, identify the fact
by a canonical injected progression Stat/query and enforce uniqueness after
Character/Choice composition, preferably through existing checked contribution
and Requirement rules. A source-local duplicate-field check cannot protect later
editing or optimization. Unknown membership must remain blocking; numeric reward
projection alone does not establish this selected-build guarantee.

## Delivery gates

1. Confirm the imported Choice-reward ownership instead of Character
   reconciliation. No public runtime model change is proposed.
2. Add numeric parameter projection to the existing reward adapter and preserve
   absent-constructor behavior. Review the valid count domain separately;
   fractional/negative or missing inputs cannot be silently rounded or defaulted.
3. Validate distinct parameter slots, units/ranges, all saved lanes, source scope,
   duplicate controls and exact Reward output before retiring a Config leaf.
4. Test independent Character/Choice selections, absent constructor, explicit
   zero and malformed inputs; refuse duplicate progression authority.
5. Reimport all five originals and verify unchanged unrelated fields, IDs and
   queries. Implement actual consumers and supplier completeness separately.

This control remains Pending until the projection and ownership checks are
implemented. No Actor Scenario workaround is published by this proposal.
