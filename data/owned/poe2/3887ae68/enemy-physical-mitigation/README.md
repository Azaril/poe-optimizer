# Enemy physical mitigation: retained reference and implementation boundary

This is an **unpublished source packet**, not an executable native consumer or
complete supplier proof. It retains five complete authenticated PoB files, nine
source spans and every one of the 39 observed Sniper Basic Attack pass outputs
from the existing physical-source witness. No new PoB execution was performed.
Full-file hashes match the pinned source manifest; existing archives are reused.
Source fields and mode names stay in this offline evidence and tests.

The [selected supplier review](source-domain-review.md) identifies nonempty
conditional Enemy baselines and later factories that must be accounted for.
Configured Armour can reuse the selected Encounter producer; shared Enemy rules
need their own reviewed applicability binding. The review is not a complete
machine-checked supplier census.

## Existing inputs to reuse

Configured Enemy Armour BASE `31e0` already has an injected native producer.
It reads raw override presence/value `31de/31df`, otherwise evaluates the real
monster table at `max(enemyLevel, 82)`, multiplies by the Pinnacle armour mean,
and rounds to one Rating. At level 82 this yields 8063. Do not import that
calculated value as a default or add another Enemy-level input. The existing
rating packet proves this producer, not complete Armour supplier membership.

Use the actual physical hit endpoints `33a0/33a1` and the forthcoming outgoing
critical multiplier. The published endpoint is pre-mitigation. Crit scaling,
other pre-mitigation multipliers and lucky hit averaging happen before armour;
source conditions can also differ between critical and noncritical passes.

## Source operation order

1. Form each pass's physical minimum/maximum after its applicable damage
   multipliers. Determine its ordinary/lucky weighted average.
2. Resolve Enemy Armour override or its accumulated value and lower bound.
   Armour-break-below-zero and its conditional override are distinct branches.
   Apply flat/full ignored armour only while armour is positive.
3. Evaluate signed armour reduction using the pass's own average and the
   `CalcArmourAsThoughDealing` factor. The source coefficient is **10**. Both zero
   operands return zero; negative armour uses its positive magnitude in the
   denominator and negates the result.
4. Full physical-reduction bypass, or chance to bypass at least 100%, sets the
   reduction to zero, including when armour would otherwise give a negative
   reduction. Otherwise combine additional physical reduction and chance to
   ignore armour, then clamp between the negative-armour bonus limit and the
   **monster reduction cap, 75%**. The Player's 90% cap is a different constant.
   Only this remaining partial/chance scaling applies exclusively to a positive
   reduction. Preserve that branch and operation order.
5. Apply damage-taken increased/MORE factors and resistance applicability, then
   multiply the pass endpoints and average by the resulting factor.
6. Combine the already mitigated critical and normal averages using critical
   chance. Sum all admitted damage types, apply hit chance, and then the actual
   rate/multiplicity rules for DPS.

The baseline physical averages are 821/1642 before mitigation and
414.2081976279727/1101.2392272188865 afterwards. Applying the critical multiplier
to the mitigated normal hit is incorrect. The retained conversion/gain controls
also show why physical-only totals cannot stand in for final all-type DPS.
These numbers are comparison targets, never native inputs or supplier evidence.
CALCS combat/buffed/unbuffed controls skip effective mitigation; they are
reference contexts, not a new native UI-mode switch. Suppressed physical damage
has absent stored-hit fields; the projection preserves that absence.

## Next blockers and admission requirements

The [Encounter/query proposal](../../../../../docs/owned-encounter-contribution-queries-proposal.md)
addresses the shared structural blocker: checked membership cannot name the
existing Encounter producer, and checked Enemy numeric queries are not yet
admitted. That proposal is awaiting review. Do not use a second configured-term
aggregator or older direct reduction as a workaround.

Before publishing an ordinary consumer, prove or implement every relevant
source family, including zero/inactive and dynamically generated records:

- Enemy Armour BASE/INC/MORE/OVERRIDE; `CanArmourBreakBelowZero`,
  `ArmourBrokenBelowZeroMax`, flat `IgnoreArmour`, `IgnoreEnemyArmour` and
  `ChanceToIgnoreEnemyArmour`.
- Enemy `PhysicalDamageReduction`, Action `EnemyPhysicalDamageReduction`,
  `CalcArmourAsThoughDealing`, full/partial/chance physical-reduction bypass and
  the chance mode's applicability.
- Physical resistance ignore flags on the attacking Action and Enemy. These
  are read even though the ordinary physical branch sets penetration to zero.
- Enemy `DamageTaken` and `PhysicalDamageTaken` INC/MORE, plus projectile,
  projectile-attack and trap/mine applicability. Retain dynamic debuff/ailment,
  party and selected encounter sources with their actual recipients.
- Critical-specific damage, lucky hit distribution, double/triple damage,
  exerted/ancestral multipliers and damage-type conversion/gain. A source output
  of one or zero is not a complete absence proof for any of these.

All input units remain explicit: Armour is Rating, raw hit is Damage,
reductions are PercentagePoints and multipliers are DimensionlessFactor.
Author normalized ratios and reference quanta through existing typed operations;
do not relabel Armour as Damage to pass type checks. Assess concrete formula
needs before proposing any new arithmetic operation.

The Rust target `owned_enemy_physical_mitigation` authenticates the source files
and exact projection, preserves missing fields, and checks the reference's
critical weighting and separate mitigation. It does **not** claim native
mitigation parity, complete Enemy effects or final DPS availability. The actual
native consumer, complete source/native writer census, joined graph tests and
all-five publication remain to be implemented after the shared contract review.

Validation: all three ordinary tests pass (0.13s), and strict targeted Clippy
passes. Logs are `runs/owned-enemy-physical-mitigation-source-01.log` and
`runs/owned-enemy-physical-mitigation-clippy-01.log`.
After strengthening the exact required body/span inventory, all three tests
passed again in the combined `runs/owned-offering-speed-rate-final-01.log` batch
(0.12s), with strict Clippy in `runs/owned-offering-speed-rate-clippy-01.log`.
