# Pain Offering: supplied Skill and scaled damage application

This published family connects physical Gem `086b` to its actual Skill `02a2`
(`PainOfferingPlayer`) using the existing supply and input projection contracts.
An explicit V15 effect application defines damage-increase delivery to the
existing Skeletal Sniper population slot `001f`. Native component tests validate
that delivery from admitted source/recipient inputs. The original build still
lacks the final-input, activation and scaling producers described below.

The 40-row `pain-offering.damage-increase` table copies the second stat in the
pinned `act_int.lua` stat set 1. Its domain is exactly integral levels 1..40.
The table values are 20..78 in steps of two, then 79..88; level 22 therefore
looks up 62. The program does not contain a special case for that level or build.
Standard quality changes radius, and alternate quality changes speed in this
source; neither is treated as a damage multiplier.

## Files and publication

The checked endpoint is `runs/owned-pain-offering-01/package`, input identity
`922ddfc5b0767d1a7938df807fe0680cfeb661cb84844dccae394ed30f6d32ec`.
Its 18 files total 60,136,032 bytes and retain 52 provenance records. Publication
adds thirteen definitions `3221`–`322d`, refines previously Unmapped Skill `02a2`
to Known with Partial declarations, and installs an explicit Partial V15
application registry. The default operations alias remains V14. This endpoint
contains no evaluation/stage bundle and does not close broader inventories.

- `skill-declaration.json` is one `DefinitionDescriptor`: the previously Unmapped
  Skill becomes Known with seven empty Partial declaration sets preserving its
  original input-schema gap. Apply it through the existing explicit release
  revision before the membership extension. The required revised release name is
  `pob-3887ae68-pain-offering-v1`.
- `extension.json` is an `OwnedRecipeExtension`. It appends the physical Gem's
  supply/grant memberships, two generated Skill parameters, thirteen new typed
  addresses in total, the finite table, and Partial rule owners. It does not
  change the predecessor's operations version or close existing inventories.
- `effective-input-binding.json` is a one-element vector of
  `EffectiveGemRecipeBinding`. Compile against the extended schema with the
  existing Rust offline compiler, then append the returned Gem-owned program.
  The physical Minion property admits the existing global level channel `30ab`;
  preparation outputs reuse exact-occurrence `30ac`/`30ad`. These remain
  pre-support values, not final generated Skill inputs.
- `applications.json` is an explicit Partial `DeclaredSet<EffectApplicationRule>`.
  Install it only when full-recipe assembly explicitly opts into V15. The default
  V14 alias and the old migration contracts remain unchanged. No stage bundle is
  provided here; any later staged evaluator must classify applications explicitly.
- `bindings.json` contains typed identifiers. `dependencies.json` and
  `dependency-slots.json` copy exact predecessor descriptors for authentication.
  They include the old Gem and Unmapped Skill, so the intentional revision can be
  checked independently of the new known declarations.
- `authoring.json` binds the exact predecessor, canonical source manifest, source
  files and the passed source witness. Publication requires that evidence gate;
  authoring data alone is not parity evidence.

Two publication tests passed, including deterministic rebuild and preservation
of all five original selections and 110 queries. Eight native tests passed;
their authenticated source join replays twelve measured cases/fifteen application
candidates through the authored programs. The fresh reference witness passed
all three tests with 37 cases/38 complete loads per JIT mode. Its canonical files
in `runs/owned-minion-physical-damage-source-02` are identical, 19,612,949 bytes,
SHA256 `030f14d71a01a2c54862eb858249b2abca1ba1caa9337dbffb3458118777e935`.
It compares every authored level-table row to the loaded source data and proves
recipient isolation with distinct quality0/20 Snipers selected independently in
MAIN/CALCS. Parsed scaling controls cover negative and fractional factors,
rounding boundaries, combined source/recipient MORE and nonneutral Magnitude.

Selected issue counts remain **116 / 116 / 108 / 121 / 19**. All five requests
remain Pending/not run, with **0/5** complete native builds. The finite component
and source evidence do not confer whole-build parity.

## Scope and missing producers

| ID suffix | Meaning and scope |
| --- | --- |
| 3221 / 3222 | Physical Gem-owned Skill supply and structural grant |
| 3223 / 3224 | Supplied Skill required final level / quality parameters |
| 3225 / 3226 | Final supported level / quality on the physical Skill occurrence |
| 3227 | Supplied Skill's admitted effect-active Boolean |
| 3228 / 3229 | Supplied Skill's resolved BuffEffect INC / MORE values |
| 322a | Supplied Skill's resolved Magnitude factor |
| 322b / 322c | Recipient actor's resolved BuffEffectOnSelf INC / MORE values |
| 322d | Recipient actor's delivered buff Damage INC contribution channel |

The Gem supply runs in the physical Skill context, reads final values `3225/6`,
and projects parameters `3223/4` into its exact generated Skill. Its true grant
literal describes the physical Gem's structural skill supply and still obeys
ordinary occurrence enablement and source-input gates. It does not activate the
buff. No program here produces `3225/6`, the effect-active Boolean, or the five
resolved scaling channels. Missing producers stay unresolved; an absent modifier
inventory cannot become zero increase, unit multiplier, or false activation.

The shared pre-support compiler handles raw level, raw corruption delta and
eligible global level adjustments. Optional physical quality can use zero only
because the predecessor's singleton quality-kind membership is complete and the
compiler checks that declaration. Final support-adjusted inputs still need their
own implementation. No direct raw SkillUse input model is introduced. Required
final quality still participates in existing source readiness even though this
particular damage calculation does not read it; that readiness contract is not
changed by this data family.

The application reads the source's final level and activation through explicit
EffectSource authority. Its resolved scaling inputs correspond to the original
BuffEffect and BuffEffectOnSelf queries, including their own membership,
applicability and store-specific rounding obligations. Future producers must
establish those resolved results; an arbitrary product of raw MORE records is
not an adequate substitute.

The actor contribution `322d` is not yet connected to a complete physical-damage
consumer. That consumer must combine separately proved passive damage, buff
delivery, parent-quality and Gigantic modifiers with the existing intrinsic
attack inputs, preserving source filtering and rounding stages. Saved usage
composition and final-input/preparation readiness remain separate pending design
decisions; this publication does not change their gates or drop the actual
Prolonged Duration support assignment.

## Scaling and stacking

For each exact source/recipient pair, the authored program computes
`scale = (1 + (source_INC + recipient_INC)/100) *
((source_MORE * recipient_MORE) * source_Magnitude)` in the source operation order.
The integer Damage entry uses `ScaleAddMod`'s two-step rounding:
`trunc(floor((base * scale) * 100 + 0.5) / 100)`.
The nodes retain multiplication by 100 and the explicit positive half bias before
flooring, including negative values. They do not replace that expression with a
quantum-0.01 nearest rounding operation, whose floating arithmetic differs near
a tie. Integer base values make the source's scale-equals-one fast path identical.
Damage has no high-precision override in the pinned `Modules/Data.lua` inventory.

V15 Maximum then groups active candidates by exact recipient, family
`pain-offering-buff`, and modifier `unconditional-damage-increase`. Copies do not
sum, negative maxima stay negative, and equal winners keep provenance. Inactive
candidates emit no contribution. Unknown activation, strength or candidate
membership cannot produce a known maximum. The application registry stays Partial.

This slice does not claim complete minion coverage: only the explicitly declared
Sniper recipient slot is admitted. Speed, radius, duration, reservation, costs,
spike survival, usage mode, alternate quality, other buffs and other recipient
families retain their gaps. Existing support assignments, including Prolonged
Duration, must remain unchanged. Finite Engine tests may supply labelled final
input/scaling facts and close a component world; those assumptions must not be
copied into production or described as whole-build parity.
