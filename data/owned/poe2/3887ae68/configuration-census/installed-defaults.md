# Installed Config defaults: remaining ownership

Source review, 2026-10-24, following checkpoint `cd176f6a`.
This is an implementation handoff, not a production census or native conversion.

## Recommendation

Of the 24 reviewed installed defaults, retain **8 responsibilities in Scenario
Assumptions** and **12 in Scenario UsageIntents** through the existing
`MissingRawRetainedScenario` target. No row requires an exact `Usage` override:
these source controls do not identify a single saved occurrence. The 12 intents
include 3 cooldown choices, 7 parent-qualified child enables, and 2 separately
reviewed target/trigger relationships. Typed native parameters, eligible source
discovery and the actual mechanics remain unresolved in those fields.

Keep the other **4 calculation-strategy responsibilities unassigned** for now.
Recommend the same ownership principle as the pending
[Calcs proposal](../../../../../docs/owned-calcs-query-ownership-proposal.md): retain the
source's reference-analysis settings durably, while native measurements request
their statistical semantics explicitly. Include these four in that existing
decision instead of asking the same ownership question twice. They are not
cosmetic Calcs focus: they change arithmetic for a fixed build and target.

Do not publish a Complete inventory from this 24-row review. It must be joined
with the full catalogue/callback, raw-leaf, outside-origin and other-owner proofs.
The current retained-Scenario receipt retires only Choice responsibility after
those checks; it emits no default, native selection, metric or gameplay result.

## Evidence boundary

Pinned PoB revision: `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`.
The source manifest is `crates/poe-optimizer-pob/data/pob-source-manifest.json`,
SHA-256 `8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675`.
The installed_defaults entries in [review.json](review.json) record the exact 24 catalogue identities, observed defaults,
raw absence, current-source hashes rechecked against that manifest, and source
anchors. The inherited source observation is the existing 23-case, fresh-load
JIT-off/on report pair; no new PoB execution or native test ran in this audit.

All 24 have no raw Input or Placeholder occurrence in authenticated Original05
(`build-05.xml`, SHA-256
`442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089`).
This does **not** mean their effective values are absent: `ConfigTab.lua:1317–1334`
installs defaults, `1169–1199` dispatches the effective values, and
`CalcSetup.lua:736–737,908–909` carries the selected inputs and modifier lists.
Visibility fields such as `ifSkill` do not guard callback execution; preserve the
actual modifier tags and calculation guards instead.

## Exact assignments

Source paths below are relative to pinned `vendor/path-of-building-poe2/src`.
Catalogue IDs refer to the authenticated ordered catalogue, not native IDs.

| ID | Source control (installed default) | Retained field | Exact consumer meaning / boundary |
| --- | --- | --- | --- |
| 0004 | `multiplierCurrentManaPercentage` (100) | Assumptions | ConfigOptions117–119 clamps Player current-Mana percentage; Skills/sup_int6802–6803 uses it with Mana to scale Fire damage. No native full-Mana assumption is emitted. |
| 0094 | `inDemonForm` (true) | Assumptions | ConfigOptions356–358 adds DemonForm condition with Life≥2 threshold. This is conditional Actor state, not proof the skill is selected or active. |
| 0105 | `touchedDebuffsCount` (10) | Assumptions | ConfigOptions381–389 emits six guarded Debuff effects: DamageTaken, ActionSpeed, FlaskChargesGained, FlaskEffect, LifeRecoveryRate and EnergyShieldRecoveryRate; all retain AffectedByGloriousMadness. |
| 0131 | `SecondsSinceInevitableCrit` (10) | Assumptions | ConfigOptions458–464 supplies elapsed time under Combat and recent-critical flags for values<4; sup_int5953,5994,6000 consumes the multiplier with separate caps. |
| 0137 | `purpleFlameStacks` (10) | Assumptions | ConfigOptions474–476 supplies capped count; CalcPerform1444–1447 creates Into the Breach chaos-gain buff only with positive count and positive maximum gain. The gain producer remains separate. |
| 0338 | `companionInPresence` (true) | Assumptions | ConfigOptions1032–1034 retains HaveCompanion guard; ModParser1835 routes the companion-presence condition. Does not select/create a companion. |
| 0449 | `conditionChampionIntimidate` (true) | Assumptions | ConfigOptions1421–1423 writes Enemy condition under Combat; ModParser2877 consumes it in an EnemyModifier Intimidated effect. Preserve Enemy identity. |
| 0503 | `GamblesprintMovementSpeed` (20) | Assumptions | ConfigOptions1609–1611 uses one of 0/10/20/30/40 under Combat and HaveGamblesprint. This is encounter roll state, not an item-roll mutation. |
| 0078 | `ConcPathBypassCD` (true) | UsageIntents | ConfigOptions320–322: exact Consecrated Path of Endurance cooldown override zero; CalcOffence410–419 consumes optional override. No resource/charge legality inferred. |
| 0110 | `FlickerStrikeBypassCD` (true) | UsageIntents | ConfigOptions398–400: Flicker Strike and source-defined transfigured variants, same override consumer. |
| 0239 | `VigilantStrikeBypassCD` (true) | UsageIntents | ConfigOptions715–717: exact Vigilant Strike, same override consumer. |
| 0167 | `raiseSpectreEnableBuffs` (true) | UsageIntents | ConfigOptions544–546: child Buff SkillType, parent summon name partially matches Spectre. |
| 0168 | `raiseSpectreEnableCurses` (true) | UsageIntents | ConfigOptions547–549: child AppliesCurse SkillType, same qualified Spectre parent. |
| 0209 | `summonCompanionEnableBuffs` (true) | UsageIntents | ConfigOptions644–646: child Buff, qualified Companion parent. |
| 0210 | `summonCompanionEnableCurses` (true) | UsageIntents | ConfigOptions647–649: child AppliesCurse, qualified Companion parent. |
| 0212 | `summonElementalRelicEnableAngerAura` (true) | UsageIntents | ConfigOptions651–653: exact Anger child SkillId and Summon Elemental Relic parent. |
| 0213 | `summonElementalRelicEnableHatredAura` (true) | UsageIntents | ConfigOptions654–656: exact Hatred child and same parent. |
| 0214 | `summonElementalRelicEnableWrathAura` (true) | UsageIntents | ConfigOptions657–659: exact Wrath child and same parent. |
| 0065 | `targetBrandedEnemy` (true) | UsageIntents | ConfigOptions288–290 plus SkillStatMap880–882: the eligible Skill's Hit targets its attached/branded enemy. Preserve the Skill→target relation; do not make this an unqualified Enemy-is-branded Boolean. |
| 0097 | `doomBlastSource` (`vixen`) | UsageIntents | ConfigOptions363 plus CalcTriggers581–607,694–710,1291–1299: selects curse expiration/replacement, Vixen trigger route or Hexblast replacement for eligible Doom Blast. Preserve actual source/target/provider relations; does not equip or grant Vixen's gloves. |
| 0025 | `resourceGainMode` (`AVERAGE`) | Unassigned strategy | Chance-weighted versus minimum/maximum resource estimates; details below. |
| 0287 | `repeatMode` (`AVERAGE`) | Unassigned strategy | Sequence average, final hit, or all hits treated as final; details below. |
| 0289 | `ChanceToIgnoreEnemyPhysicalDamageReductionMode` (`AVERAGE`) | Unassigned strategy | Expected versus extremal stochastic mitigation; details below. |
| 0111 | `elementalConfluxElement` (1: Average) | Unassigned strategy | Average distributes the three elemental effects rather than selecting one world state; details below. |

### Why the 12 Skill responsibilities use intent

The three cooldown choices apply to every eligible exact occurrence and may be
dormant. Source `SkillName` matching (`ModStore.lua:892–926`, with skill/summon
configuration at `CalcActiveSkill.lua:675–689`) must eventually become finite
owned definition/provider applicability, never native name matching.

The seven enables write `SkillData.enable`. Source preserves the parent at
`CalcActiveSkill.lua:144–152`, matches parent/child constraints through
`ModStore.lua:892–931,949ff`, and extracts matching SkillData at
`CalcActiveSkill.lua:895–901`. `CalcPerform.lua:2586–2592,2634,2735` consumes that
enable on minion child Buff/Aura/Curse paths with additional mode/target guards.
Their effect application recipients and stacking remain independent. The receipt
must not create a missing child, enable its parent, or call an incomplete child
census empty.

The two relational settings also fit this existing **responsibility owner**:
the global Config value does not identify one occurrence, so exact Scenario
`Usage` would invent a target; a global Actor assumption would lose the skill
relation. Their future native conversion still needs exact eligible definitions
and actual relationships. For Doom Blast, the source even warns when Vixen mode
is selected without the gloves (`CalcTriggers.lua:695–696`); retaining the intent
does not prove the provider exists or bless PoB's title-based test. No new native
selector/trigger API is required merely to retain the unresolved responsibility.

## Four arithmetic strategies: exact evidence and one ownership choice

1. **Resource gain.** ConfigOptions192–198 emits `AverageResourceGain` or
   `MaxResourceGain`. ModParser5053–5055 defines chance-based LifeOnHit as
   `amount*chance/100` versus the full amount; 5072–5075 similarly chooses chance
   percent versus 100% of Life on kill. Minimum omits the chance contribution.
   This is a statistic over outcomes, not a different current Life assumption.
2. **Repeat evaluation.** ConfigOptions866–879 installs average/final flags for
   Player and minions. CalcOffence1138–1233 weights repeat-area, double-damage and
   damage terms differently for average versus final evaluation; 3161–3163
   applies a distinct final-only DPS adjustment; 3831–3847 selects average final
   repeat critical chance versus certainty. `FINAL_DPS` is not the same as
   `FINAL`. The source's `"None"` spelling at3832 versus UI `"NONE"` is an
   independent source discrepancy, not permission to normalize native semantics.
3. **Physical reduction chance.** ConfigOptions881 supplies MIN/AVERAGE/MAX.
   CalcOffence4304–4316 replaces a chance strictly between 0 and100 with 0 or100
   for extrema, otherwise keeps the chance in the weighted physical reduction.
   It changes the reported estimator with the same enemy and attack.
4. **Elemental Conflux.** ConfigOptions401–419 uses default1 (Average) to supply
   all three inverse multipliers as3; a selected element instead gets1 and the
   others0. Skills/act_int5651–5654 applies those inverse multipliers to the
   Lightning/Cold/Fire MORE Buff records. The default represents a mixture, not
   one simultaneous physical elemental state. Averaging modifiers is source
   behavior; it is not automatically a mathematically correct expected final
   damage calculation for nonlinear downstream operations.

**Recommended ownership choice:** native analysis policy belongs with the
requested measurement/query semantics. Preserve these four PoB choices as
authenticated reference-analysis state until an explicit native conversion is
requested and implemented. This is the same boundary proposed for Calcs, extended
to these finite arithmetic controls; it does not introduce another Scenario
assumption category just to retire their current obligation.

Current `MetricRequest` is `{id, metric, target}`
(`owned_build/records.rs:478–482`), and the checked missing-raw target admits only
the three actual Scenario fields. **Use existing injected MetricDefId/result
semantics first**: separate expected, conditional and extremal metrics can carry
different measurement meanings without a new mode field. The absence of a mode
string on MetricRequest is not a model defect and does not justify adding a
framework. The actual open decision is the Import treatment of these reference
choices, not whether native Core needs a new strategy type. Keep this decision
in the existing Calcs review: ask
whether reference analysis choices remain source-owned while native measurement
semantics are explicit, rather than separately asking permission for four
generic Scenario routes. If automatic import projection is preferred, the
Calcs proposal already requires real metrics/targets and complete assumptions;
these four add actual estimator correspondence to that work.

Durable source retention is required, including pinned source/default semantics
for absent fields. An XML hash alone does not preserve installed defaults, and
omission equivalence cannot prove harmlessness. Until that accepted contract and
its checked disposition exist, the four stay Pending; do not route them through
`MissingRawNoAdditionalInput` or `MissingRawRetainedScenario`.

## Next executable boundary

- Use the existing checked retained-target shape for the 20 reviewed assignments
  when assembling the independently complete inventory. Preserve the actual
  same-Scenario Pending fields/codes and all known members; no blanket Scenario
  or Choice completion from this matrix.
- Keep the four strategy rows explicitly unresolved and bring the finite source
  evidence into the existing Calcs ownership decision. The 482-row absent-source
  review is disjoint; these24 are its exact exclusions.
- Acceptance must show all-lane raw absence, exact independently named controls,
  same-scope owners, retained conditions/relations and unchanged native data.
  Nonempty raw false/zero/empty values must not use these missing-raw routes.
  Full census, global-origin coverage, replay and atomic-budget laws remain the
  existing mandatory gates.
