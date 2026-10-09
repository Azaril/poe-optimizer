# Draft Player Mana consumer

**Not published.** This arithmetic fragment is being validated using existing
typed rules. It must not enter the canonical release before the
[numeric-domain decision](../../../../../docs/owned-numeric-domain-proposal.md)
and its admission checks are implemented. It establishes no complete-build
coverage and adds no acquisition or runtime backend.

The program reads the existing intrinsic/inherent Mana groups, reward/passive
increase groups, MORE group, five typed adjustment collectors and coherent
override selection. All game values and formulas remain in the injected data.
It derives the existing Mana stat `29f9` through a Player receiver.

The arithmetic order is:

1. Sum base contributions and increase groups independently.
2. Sum the three outgoing conversion percentages and cap their combined value
   at 100%; do not introduce a lower clamp.
3. Retain that fraction of base Mana, then add pre-scaling additional Mana.
4. Apply the increased/reduced factor, then the MORE/less multiplier.
5. Add post-scaling Mana, round, then apply the ordinary minimum of one.
6. A known override, including zero, bypasses ordinary arithmetic. Unknown
   override presence remains unavailable, and whole-request source coverage
   remains mandatory even when a scalar branch is unused.

The formula uses ordinary `Add`, `Scale`, percentage conversion, `Round`,
`Maximum` and lazy `Select` operations. It does not copy a Lua callback or add
a resource-specific Rust kernel. The pending domain check must sit on the
actual pre-rounding operand, with an explicit unsupported result outside the
proved range; it cannot be an effect-inactivity guard or game Requirement.

`tests/owned_intelligence_mana.rs` composes this fragment with the real native
level, attributes, reward and override producers in the existing finite
component fixture. A separate scalar-boundary test injects all retained
original-source operands and compares source results; that test does not
claim those supplied aggregates are native producer implementations.

Validation: all **19 ordinary tests** in that existing target pass. The joined
fixture derives **638 Mana** from its real producers, responds to level and
attribute changes, and retains a present zero override. All **16** retained
source vectors match in forward/reverse order. Missing and Partial dependencies,
stage ordering and fresh/reused/four-worker comparisons remain checked. Evidence:
`runs/owned-mana-pool-draft-native-05.log`. These finite controls do not prove the
unbounded computed domain or turn the draft into a production release.

Resource transformations into the Extra/Total channels, item-granted origins,
global Partial closures and the original build's outstanding input obligations
remain separate work. The canonical package remains the preceding Mana override
publication until a checked successor is deliberately assembled.
