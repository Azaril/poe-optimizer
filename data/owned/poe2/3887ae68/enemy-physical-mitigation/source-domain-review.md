# Selected Enemy mitigation supplier review

Date: 2026-10-15. **Read-only investigation; not a complete source-admission
checker, native supplier declaration, or publication gate.** No source VM or
Cargo execution was performed for this review. Original05's exact Sniper Basic
Attack remains the selected scope.

## Finding

Existing configured Armour BASE producer `31e0` is the only selected **initial
Enemy Armour BASE** supplier found. It is the right producer to reuse, not a
reason to import the observed 8063 as a constant. Full Armour membership still
needs the late-route exclusions below and an actual native writer census.

Enemy damage-taken and physical-reduction channels are **not empty**. They have
shared conditional baseline suppliers, independent of Encounter configuration.
The current shared-Actor binding cannot own rules on Enemy. This ownership gap
must remain explicit rather than copying shared baseline rules into every
Encounter definition.

## Retained input boundary

Both complete `mana-supplier-sources/source-jit-{off,on}.json.gz` reports were
inspected recursively, including nested records and zero/inactive values, across
18 baselines, two configuration constructions, 125 actual callback invocations,
21 environments, 2394 node constructions, 42 node lists and 1536 skill
constructions. The 663-entry configuration catalogue was not mistaken for
executed callbacks. These captures are construction evidence, not a final Enemy
store trace.

Expanded hashes are `542e0e7ebc1947a45be14f8b0c9e4eed28df2d37d7d46c8d3fd0a1ee5444c971`
and `fd7cd3dfbe854e876fa9bc452d09814edefbd53d83019869bd0e06da73ceb22e`.
Both have the same ten distinct raw records whose names contain Armour,
PhysicalDamageReduction or DamageTaken. Repeated capture appearances are not
additional game suppliers. Full retained CalcSetup, CalcPerform, CalcOffence,
ModStore and PartyTab bodies establish the routing and later factory boundaries;
their existing packet/manifest authorities remain the source of file identity.

## Concrete nonempty sources

| Owner and recipient | Potential contribution | Source authority |
| --- | --- | --- |
| Shared baseline on Enemy | DamageTaken INC 10 when Intimidated | CalcSetup 75 |
| Shared baseline on Enemy | DamageTaken INC 10 when not Intimidated and Party:Intimidated | CalcSetup 76; avoids stacking duplicate causes |
| Shared baseline on Enemy | Spell-only DamageTaken INC 10 when Unnerved, or guarded Party:Unnerved | CalcSetup 79-80; Basic is an Attack, not a Spell |
| Shared baseline on Enemy | PhysicalDamageReduction BASE -15 when Crushed | CalcSetup 106 |
| Selected Encounter on Enemy | Armour BASE from configured raw override/default | ConfigOptions 2183-2187; source02 callback indexes 43 and 106; native `31e0` |
| Shared baseline on both combatants | ShockBase 20 conditioned on the other combatant being Shocked; ShockStacksMax 1 | Exact baseline raw stores; later Shock factory must be reviewed |
| Shared baseline on Enemy | Adrenaline FLAG true conditioned on Adrenaline | Exact Enemy baseline; CalcPerform 674 can later create physical reduction |
| Selected Encounter on Enemy | ShockStacks 1 under Effective | Both actual configuration lists; stack count alone does not activate Shock |

Do not merge Player-only sources into this domain: Glorious Madness contributes
conditional DamageTaken INC 60 to Player; the saved Eye of Hinekora and Qimah
rewards contribute Armour INC 30 and 15 to Player. The item Body Armour quality
multiplier is also not Enemy Armour. Friendly minion intrinsic Armour and
Player/minion buffs have different recipients.

CalcSetup 903-912 initializes the shared Enemy baseline first, then adds selected
configuration and Party lists. This separates mechanical ownership from import
origin. Configuration is not the owner of the shared baseline formulas.

## Condition authority

ConfigOptions 1896-1906 has separate Enemy Intimidated, Crushed and Unnerved
callbacks. None is invoked in either selected configuration construction; no
corresponding raw Condition flag or direct condition/override is present in the
reviewed input closure. The present `Condition:ChampionIntimidate` marker is a
different flag. It requires an actual supplier such as the parser's nested
EnemyModifier to activate Intimidated; it does not imply that condition itself.

The recursive selected construction scan found no EnemyModifier, ExtraCurse,
ExtraAura, AffectedByAuraMod, AbyssalWasting condition/delivery, or target
Intimidated/Crushed/Unnerved supplier, including nested inactive records. The
40 selected skill recipes have empty callback inventories. Complete constructed
buff lists contain Offering, Reaver Rage, and Frost Bomb's elemental exposures;
they do not contain physical mitigation modifiers or those conditions.

Party:Intimidated and Party:Unnerved are source import namespaces, not additional
native combat conditions. PartyTab 717-725 translates incoming Party condition
names, and CalcSetup 912 imports that list into Enemy. The retained
`source-empty-party` proof establishes a fresh, childless saved Party; all 21
source02 environments also retain empty Party records. Reuse the original proof,
not a new inference from an empty filtered query.

ModStore 409-414 resolves an explicit query override first, then direct and
parent condition state, then the matching Condition flag. Negative condition
admission therefore needs all these source paths, exact selected source guards
and the native complete candidate census. A missing native producer is not false.

## Later factories and remaining proof boundaries

| Boundary | What must remain explicit before mitigation publication |
| --- | --- |
| CalcPerform 565-578, 1258-1262 and 845 | Generic EnemyModifier delivery from Player, selected minion and Enemy; destination is each actor's actual enemy. Current raw source closure has no candidate, but recipient identity must be checked. |
| CalcPerform 3131-3147 | Debuff/curse lists are delivered to Enemy. Actual Frost Bomb elemental exposures are present; do not label the entire debuff channel empty. Their target stats do not change physical mitigation. |
| CalcPerform 727-742 | Fully broken armour can add PhysicalDamageTaken, or all hit DamageTaken. No selected ArmourFullyBroken condition or enabling conversion flag was found. |
| CalcPerform 671-674, 828 and 3356-3362 | Enemy Adrenaline can add physical reduction; Malediction and consecrated-ground effects can add DamageTaken. Adrenaline's conditional baseline FLAG exists, but no activating direct/Condition state was found; the other enabling sources are absent in the reviewed closure. |
| CalcPerform 3273-3342 | Shock is a dynamic DamageTaken supplier, including a possible zero-valued inactive record. No selected ShockOverride/Minimum/Val or Shocked condition was found, but baseline ShockBase and configured stack count are present. |
| CalcPerform 3264-3268 | Bonechill is cold-only; ChillEffectIncDamageTaken could affect physical damage through generic DamageTaken. Its enabling source is absent in the reviewed closure. |
| CalcPerform 1264-1275 and 3414-3427 | Abyssal Wasting can generate condition names and arbitrary effects. Its selected enabling/delivery records are absent. |
| CalcPerform table-driven legacies | LegacyOfBasalt has Armour INC 150 and LegacyOfGranite Armour BASE 2000. No selected LegacyFlask/legacy source was found; table-driven factories must be covered, not just NewMod name scans. |
| CalcOffence 4295-4299 | CanArmourBreakBelowZero can create an Armour OVERRIDE conditioned on ArmourBrokenBelowZeroMax. Neither source condition is present in the reviewed closure; preserve the branch as unsupported outside that finite domain. |
| CalcOffence 4300-4317, 4374-4387 | Armour ignore, physical-reduction bypass, calculation magnitude, and physical-resistance applicability need their own exact selected Action/Enemy candidate census, including zero and inactive suppliers. |

The Shock outer gate at CalcPerform 3302-3304 uses an un-compared numeric Sum
inside `or`. Lua treats zero as true, so an empty effective sum does not prove
the factory was skipped: it can append conditional DamageTaken zero. Establish
the semantic inactive/zero source domain without exporting Lua truthiness into
native rules. This is distinct from accepting a missing source as neutral.

Initial source records and empty recipe callbacks substantially narrow these
boundaries; they do not replace a machine-checked late-writer proof. No new VM is
currently justified: first author exact source/candidate checks from retained
inputs and complete bodies, and identify a specific missing observation if any
boundary cannot be established that way. Existing physical-source raw modifier
observations are filtered for physical damage construction and are not a complete
Enemy mitigation census.

## Native contract and shortest next steps

The October 16 Actor cutover resolves the earlier contract gap: Player, supplied
units and ScenarioEnemy share one Actor execution context. Shared local rules
bind explicit eligible roles and exact occurrences; checked contribution origins
include the selected Encounter. No placeholder Player or distinct mechanical
Enemy execution kind remains. This implementation does not itself close any of
the source families reviewed above.

The finite native armour consumer now reuses `31e0` through the checked query
graph, with separate normal/critical averages. Retain its independent
override/transform/bypass obligations. Use the implemented shared Actor ownership
when delivering conditional reduction and
damage-taken contributions. Keep actual sources, typed condition provenance and
the Intimidate non-stacking rule; retain Spell applicability for Unnerved. Then
combine these inputs with separate normal/critical physical hits and the actual
signed armour formula. Every publication still requires exact native potential
writers, explicit unsupported-domain guards, unchanged Partial/global coverage,
joined numerical controls, deterministic scratch/Rayon evaluation and all-five
import preservation.
