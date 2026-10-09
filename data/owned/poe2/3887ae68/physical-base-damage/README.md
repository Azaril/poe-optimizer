# Draft physical base damage consumer

**Unpublished arithmetic fragment.** This packet adds no canonical package,
producer inventory, build admission or coverage closure. Its ordinary Rust
tests compose it with the existing finite Sniper replay through the public
native definition, rule, stage and evaluation APIs. It contains no Lua code,
callbacks or runtime subprocesses.

One generic Action program is bound by data to Basic Skill `0021` / output
`0022`. For each minimum and maximum endpoint it evaluates, in this order:

```text
((intrinsic + bonus) + ((self_flat + enemy_flat) * added_multiplier))
    * base_coefficient
```

There is no rounding, clamp, implicit zero or implicit identity multiplier.
The existing `3212` / `3213` inputs are intrinsic endpoints, already produced
and routed by native rules in the finite replay. The intrinsic range is not
multiplied by the profile's `1.15` again. `336e` / `336f` are the resulting
physical base endpoints before damage conversion and subsequent hit scaling.

All ten inputs are required resolved Action quantities. The damage inputs and
outputs use unit `1d3a`; the two factors use unit `0001`. Reserved draft Stats:

| IDs | Meaning | Production state |
| --- | --- | --- |
| `336e`, `336f` | Minimum / maximum physical base output | This draft consumer |
| `3370`, `3371` | Bonus endpoints on the selected damage source | Producer unfinished |
| `3372`, `3373` | Eligible self flat minimum / maximum | Checked collection unfinished |
| `3374`, `3375` | Eligible enemy flat minimum / maximum | Checked collection unfinished |
| `3376` | Combined added physical / generic damage multiplier | Checked collection unfinished |
| `3377` | Final base damage coefficient | Producer unfinished |

These operand declarations are a consumption boundary, not proof that the
operands exist. Bonuses belong to the selected damage source, which need not
always be an Actor. The coefficient must account for the selected skill-level
coefficient and effective SkillData; the level table alone cannot establish
its value. In particular, SkillData application at both CalcActiveSkill:896–900
and CalcOffence:738, and the SkillStatMap's base-multiplier mapping, remain
relevant upstream. An individual
intrinsic AddedDamage contribution is not a complete combined multiplier.
This fragment adds no unchecked Enemy fold or new public query contract.

`bindings.json` pins the existing intrinsic-added-attack-damage source vectors.
Those retain original CalcOffence observations from both source02 JIT modes,
including 13 executed Basic occurrences across the ten recorded cases.
Unexecuted CALCS occurrences are not observations. The source formula is
CalcOffence lines 4131–4138 at the recorded revision.

The Engine tests read the pinned source operands as finite test inputs, and
compare native endpoints and final arithmetic directly with recorded source
outputs. Only these records establish absent bonuses, zero enemy additions,
the combined multiplier `1.15` and coefficient `1`. They do not authorize
zero/one production defaults. Synthetic signed, zero and fractional controls
exercise arithmetic, operation order and exact occurrence separation; they
are not evidence of game legality or unbounded numerical parity. Existing
finite-quantity failure behavior remains in force; the pending numerical
domain decision is not resolved by these tests.

The packet retains the Action owner's Partial declaration. Test composition
preserves all predecessor closures and adds explicit scheduling without
changing the tracked replay artifact. One fixture-only literal program supplies
the eight operands solely in the Rust test helper. With any required input unavailable,
the affected endpoint stays unresolved, including when a known coefficient
is zero. No complete build, final hit, average hit, DPS, conversion or defence
result is claimed.

Validation: all five new native tests passed across the full target run and
one focused repair run; all 21 preceding tests passed in the full target run.
Strict target Clippy also passed. Logs are
`runs/owned-physical-base-draft-native-01.log`,
`runs/owned-physical-base-draft-native-02.log` and
`runs/owned-physical-base-draft-clippy-01.log`.
