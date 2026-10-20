# Enemy physical mitigation: ordinary armour component

This packet contains an executable native **partial armour component**, together
with its retained reference evidence. It is not complete physical mitigation or
a complete supplier proof. It retains five complete authenticated PoB files, nine
source spans and every one of the 39 observed Sniper Basic Attack pass outputs
from the existing physical-source witness. No new PoB execution was performed.
Full-file hashes match the pinned source manifest; existing archives are reused.
Source fields and mode names stay in this offline evidence and tests.

The [selected supplier review](source-domain-review.md) identifies nonempty
conditional Enemy baselines and later factories that must be accounted for.
Configured Armour reuses the selected Encounter producer through the generic
Actor graph. The Actor migration already provides role-specific applicability
and exact Encounter membership. The source review is not a complete
machine-checked supplier census.

## Existing inputs to reuse

Configured Enemy Armour BASE `31e0` already has an injected native producer.
It reads raw override presence/value `31de/31df`, otherwise evaluates the real
monster table at `max(enemyLevel, 82)`, multiplies by the Pinnacle armour mean,
and rounds to one Rating. At level 82 this yields 8063. Do not import that
calculated value as a default or add another Enemy-level input. The existing
rating packet proves this producer, not complete Armour supplier membership.

The consumer uses the actual physical hit endpoints `33a0/33a1` and outgoing
critical multiplier `33ab`. The hit endpoint is pre-mitigation. Crit scaling,
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

The shared graph now names the actual Encounter producer and ScenarioEnemy
Actor. The consumer adds normal and critical armour-adjusted physical averages
`33b2/33b3`, one program, and four checked Armour queries. Their production
membership remains **Partial**, as does the existing Action owner. No evaluation
bundle or complete-build admission is added.

The finite numerical tests supply a deliberately closed test fixture and run
the actual configured Armour table producer, physical hit and critical programs.
They prove the ordinary BASE/INC/MORE and optional override calculation,
separate critical mitigation, signed armour, zero denominator and monster cap.
They also reject missing inputs, incomplete membership, unlisted zero/inactive
writers and late producers, and compare fresh, reused and Rayon results.
These fixture closures are never promoted to production supplier evidence.

Before complete mitigation or final DPS admission, prove or implement every relevant
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
critical weighting and separate mitigation. `owned_enemy_physical_mitigation_native`
runs the actual typed consumer; `owned_enemy_physical_mitigation_cli` checks the
publication and preservation contract. These tests do **not** claim complete
Enemy effects or final DPS availability. Candidate-bound supplier admission and
the remaining branches above stay blocking.

Validation: all three ordinary tests pass (0.13s), and strict targeted Clippy
passes. Logs are `runs/owned-enemy-physical-mitigation-source-01.log` and
`runs/owned-enemy-physical-mitigation-clippy-01.log`.
After strengthening the exact required body/span inventory, all three tests
passed again in the combined `runs/owned-offering-speed-rate-final-01.log` batch
(0.12s), with strict Clippy in `runs/owned-offering-speed-rate-clippy-01.log`.

## Native execution and publication, October 19

The six native armour controls and three publication controls pass. The native
target also retains all nine outgoing-critical controls. The explicit all-five
publication passes in 33.29 seconds, preserves all 110 queries and exact unrelated
fields/local IDs, and rebuilds byte-for-byte. Selected issue counts remain
106/117/109/123/4; complete original builds remain **0/5**. The new four query
groups and existing owner closure stay Partial.

Validated local package:
`runs/owned-enemy-armour-component-publication-01/package`.

| Identity | Value |
| --- | --- |
| Prior input | `a7e1cb77b85ecd34ff18ea7b99cf104b41079cd9328715e00553d2785c1f9e3d` |
| Input | `5b611f43a0eedbf556799bcbd2c295a85d58d3264a7306c24602fd4d41e6e7b6` |
| Definitions | `eccce76d53d38ad33da0438942b3d282397d04f7a2385d40dbab7b0fda5b2ac5` |
| Rules | `144eba8004170582f7e01e0f27e90a9e9347b774e486c1afe1666334c6540913` |
| Compiled rules | `82e205b3d1332427c168f432b9445b90939541b14a0ab024eb69f39399ba0aff` |

The package has 8206 effective programs, with the last allocated Stat `33b3`.
Logs: `runs/owned-enemy-armour-component-tests-02.log` and
`runs/owned-enemy-armour-component-publication-01.log`. The first post-rebase
test run rejected integer versus decimal JSON spelling in a table comparison;
the corrected test compares the entire typed table and exact required inventory.
Source bytes, hashes, units and every row remain checked.
