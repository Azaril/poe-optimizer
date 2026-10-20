# Proposal: numeric progression through existing build rewards

**Status:** Accepted and implemented locally on 2026-10-19. Numeric import/accounting, actual campaign rules, same-attempt authority assessment and all-five publication pass. Complete candidate admission remains unfinished.
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

1. Accepted: imported Choice-reward ownership instead of Character
   reconciliation. Reuse the existing runtime reward model.
2. Add numeric parameter projection to the existing reward adapter and preserve
   absent-constructor behavior. Review the valid count domain separately;
   fractional/negative or missing inputs cannot be silently rounded or defaulted.
3. Validate distinct parameter slots, units/ranges, all saved lanes, source scope,
   duplicate controls and exact Reward output before retiring a Config leaf.
4. Test independent Character/Choice selections, absent constructor, explicit
   zero and malformed inputs; refuse duplicate progression authority.
5. Reimport all five originals and verify unchanged unrelated fields, relationships
   and queries. A new Reward occurrence can shift later generated local IDs:
   validate exact source/typed-target correspondence across that shift instead of
   assuming a constant offset or retaining an obsolete importer. Implement actual
   consumers and supplier completeness separately.

The real projection policy and ownership rules are now published locally in
`runs/owned-campaign-progression-publication-02/package` (input `c9d047cc…`).
The five original counts are preserved; the configuration leaf is accounted for,
while broader input inventories remain Pending. No Actor Scenario workaround
is published. Complete consumer/source coverage is separate from this component.

## Implementation handoff, October 19

Keep the bounded production changes in Import: extend `owned_reward_policy.rs`
with explicit finite-case versus numeric-parameter projection outcomes, and
update the current wire shape in place. A projection declares its Reward,
parsed-value parameter slot and fixed parameters. Existing schema checks must
validate owner, slot, parameter site, required assignments, kind, units and range.

The implementation uses one typed finite-case/numeric-projection union. Existing
finite outcomes remain an array under `outcomes`; a numeric projection is an
object under that same field. Both are current supported operations, not old/new
compatibility branches. This keeps unchanged finite policies and their identities
stable while every numeric projection has its own content identity. No versioned
fallback reader, source-specific native type or second reward pipeline is added.

Update normalization's reward emission, reward-inventory proof and configuration
dispositions together so selected and unselected source origins remain accounted
for; their current Boolean/Option and single-selector assumptions are insufficient.

This control uses the source's `count` selection: Input zero with Placeholder 56
selects 56. Plain recipe tier precedence would select zero. Inspection of pinned
`ConfigTab.lua:1183–1187` confirms that both Input and Placeholder zero skip
application. The existing scalar `NonzeroInputThenPlaceholder` projection only
filters Input zero, because it serves a different reader meaning. Keep the reward
selection explicit: select the first nonzero numeric candidate, preserve all
malformed losing lanes and duplicates, and leave a missing or all-zero selection
unresolved. Do not conflate either behavior with the native progression fact.
Generic numeric projection can still accept an explicitly authored zero when
its schema and source-selection policy admit it; this source control supplies no
such authority through a skipped callback.

No new runtime authority mechanism is needed. Each progression Reward can emit
the canonical fact as an Override and an integer authority contribution of one.
A later Reward-owned validation program reads a complete `ExactIntegerSum`
authority query and emits a Requirement for exactly one occurrence. This detects
equal-valued duplicates that `RequireAgreement` alone would accept. The same occurrence
referenced from both presets remains deduplicated by normal composition. Keep
the Requirement on the Reward: shared existing-Actor rules do not have that
permission. Unknown membership and missing required facts remain unavailable.

**Admission follow-up discovered during implementation:** native effect execution
currently exposes Requirement as a Boolean effect. A false Requirement is not by
itself proof that the complete evaluation/optimizer rejects the candidate. The
progression gate therefore needs both the exact rule result and a final admission
consumer: false must mean invalid, unavailable must remain incomplete, and every
selected requirement must be accounted for. Validate that seam before claiming
post-composition uniqueness is enforced. Do not add a progression-only validator
or treat an `Evaluated` effects report as complete build validity.
The [report contract proposal](owned-requirement-assessment-proposal.md) was
accepted October 19: requirement assessment accompanies native metric reports.
Its production collector passes ordinary/support and exact Reward tests.
Final admission integration remains a gate; a passing partial component is not
a complete gameplay validity certificate.

Adding a Reward during ConfigSet normalization shifts subsequently allocated
local IDs, beginning with its Scenario preset. The focused multi-ConfigSet test
checks the existing allocation behavior; it does not provide the all-five proof.
Use checked source-to-typed-target correspondence at the real publication, including
all references and query targets. Scalar-assumption publication's no-new-occurrence
inverse cannot be copied as proof of this case, and a bare numeric offset is not
an identity proof. No stable-ordinal guarantee across changed imports is added.
The real all-five publication now proves injective correspondence over whole
drafts/sidecars, exact selections and unchanged query bytes. Six source controls
also check selection, zero/absence and malformed losing lanes. Exact identities,
logs and remaining blockers are in the living implementation checkpoint.
