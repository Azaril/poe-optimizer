# Final coefficient for the intrinsic Basic attack

This offline proof addresses only `MinionMeleeBow`, supplied by the pinned
Skeletal Sniper profile. It establishes that its final base-damage coefficient
is one across the normal imported-input domain of this release. It does not
claim complete damage, final metrics or build coverage, and it does not make one
a fallback for an unknown skill or unknown producer. The resulting native value
belongs in injected data for this exact ability; no Lua key or table behavior is
required in the runtime contract.

## Original selection and level inputs

The already retained `minion-attack-selection` packet establishes the intrinsic
source, exact provider domain, callback absence and allowed stat-map metatable.
Its bytes remain unchanged and are pinned by this packet. The new acquisition
uses the same bounded original catalog constructor and complete raw-table walk.
The original ModCache is loaded in the same restricted host to inspect cached
modifier records. It does not invoke parser callbacks with guessed arguments.

At CalcOffence.lua 4131, the source precedence is the selected granted-effect
level field, then effective SkillData, then one. CalcActiveSkill.lua 795–808
copies the active definition's level row and overlays the selected stat-set row.
The constructed Basic definition has exactly one level and one stat set, both
without baseMultiplier. CalcTools.lua 60–76 validates a requested level against
that declared level domain. Changing actor level, quality or support levels does
not create another declared Basic row. The packet retains the actual level,
quality, alternate-quality, stat-set, base-modifier and callback inputs, rather
than inferring them from the example build's saved level.

## Effective modifier supplier domain

The complete constructed catalog and cache have exactly one SkillData record
whose key is baseMultiplier. It is the global mapping for
`active_skill_has_%_standard_scaling_attack_damage` at SkillStatMap.lua 2176–2178.
The original mapping divides its stat input by 100 and has no GlobalEffect tag.
There are no direct or nested catalog/cache coefficient modifier suppliers.

The actual constructed inputs to CalcTools.buildSkillInstanceStats, including
every skill's quality/alternate-quality rows and all stat sets' indexed/constant
stats, identify six consumers of that mapping:

| Definition | Constant percent |
| --- | ---: |
| ArmourExplosionPlayer | 30 |
| TriggeredBrambleslamPlayer | 35 |
| TriggeredCraterPlayer | 35 |
| StompingGroundShockwavePlayer | 50 |
| TriggeredVolcanicEruptionPlayer | 20 |
| TriggeredCaltropsPlayer | 75 |

All six are separate active effects; none is a support definition. Their location
in a file named sup_str/sup_dex does not grant them support ownership. Basic has
no matching stat input. CalcActiveSkill.lua 731–777 imports support modifiers
only from definitions with support=true; granted triggered attacks have their own
active occurrence. The map has no global-effect tag, so it cannot leak from a
different skill through cross-skill buff extraction at CalcActiveSkill.lua
1032–1105 or ExtraCurse filtering at CalcPerform.lua 2933–2951. The same-definition
stat-set global filtering at CalcActiveSkill.lua 69–140 also retains no such
coefficient supplier. Data.lua 72–102 adds provenance and actor conditions rather
than inventing a global-effect tag. Explicit MinionModifier payload transport at
CalcPerform.lua 1161–1167 and ExtraEmpowerMod at CalcActiveSkill.lua 400–420 are
covered by the nested modifier inventory; neither contains a coefficient payload.

ExtraSkillStat is a separate possible route into the selected stat map at
CalcActiveSkill.lua 81–85 and 795. The packet captures its actual cached records
and the complete literal fallback-factory key inventory. None supplies this game
stat. A computed factory key requires renewed review rather than being assumed
unrelated. Normal custom modifier text uses that same parser; configuration
callbacks and saved skill import cannot insert an arbitrary SkillData table.

Formatted party exports are a separate raw-input boundary, not ordinary modifier
text. PartyTab.lua 516–543 imports these records through ModTools.lua 123–141.
Their modifier names and types are authored, but their value is restricted to
Boolean true or a number, with zero for an unparsed value. They can therefore
supply arbitrary numeric channels such as AddedDamage; the ordinary catalog and
parser census alone does not close those channels. They cannot construct the
table payload required by SkillData (`key`, `value`) or ExtraSkillStat. A scalar
record labeled as either LIST family is malformed and remains unsupported; it
must not be counted as a successfully evaluated empty coefficient supplier set.
Saved Party and ImportedBuffs source rows retain live Pending import obligations.
This coefficient result neither admits party input nor closes those obligations.

Existing SkillData transport must still be accounted for: CalcActiveSkill.lua
896–900 applies environment then skill LIST records; CalcSetup.lua 2218–2224
rebuilds them; CalcOffence.lua 738–744 applies the skill records again. The pinned
source review and raw catalog proof cover parent/minion/global/ExtraSkillMod
transport and source callbacks. Transport copies or scales existing records; it
does not invent the missing coefficient key or game-stat supplier. Basic's own
execution callbacks remain absent. This is a reviewed claim for the exact pinned
source, not a general interpreter or proof of arbitrary future Lua.

## Native meaning and validation

For this exact ability both higher-priority supplier domains are empty, so the
final coefficient is the authored value one. No native last-write ordering,
truthiness rule or optional numeric-selection system is needed. Future abilities
with real declared or computed coefficients must use their own data-defined
composition. Unknown applicability is not equivalent to an empty supplier set.

The original preconversion source03 report corroborates coefficient one in nine
executed Sniper contexts, with both raw and eligible supplier records retained.
It includes flat-damage and conditional controls and distinguishes unexecuted
CALCS contexts. Those observed outputs are not the supplier-completeness proof.

The new tests reconstruct the actual catalog and cache and compare the compact
artifact exactly. Constructed-source negative controls insert a zero coefficient
in a level or stat-set row, a quality-derived coefficient, a dormant zero modifier,
a selected callback and a parser ExtraSkillStat supplier. They recompute source
hashes and still fail the semantic gate. Zero and inactive sources therefore
cannot be mistaken for absence.
