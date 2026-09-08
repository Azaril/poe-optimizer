# Native calculation engine and browser target

This document defines the native replacement track. Delivery status, validation results
and the next resume point belong in [implementation.md](implementation.md). The Rust
engine develops alongside the mlua/PoB evaluator: the latter supplies a versioned oracle
and usable evaluation path while the native engine grows through verified slices.

## Boundaries and intended outcome

`poe-optimizer-engine` owns game calculations that can run without Lua or an operating
system. It accepts resolved numeric inputs, validated numeric modifier layers, and explicit condition contexts now,
and the current Spark/Mace pipelines accept injected, immutable compiled game data.
The independent `poe-optimizer-data` model owns content and balance parameters. The engine
owns operation semantics and compiles supported records into borrowed calculation views;
see the [data-boundary decision](game-data-boundary.md). Most game values must be loaded
from configuration, including patch-dependent formula coefficients. Compatible value/table
changes must not require Rust edits; new operations still require code and parity review.

The optimizer core owns objectives, constraints and search;
portable import owns interchange and materialization, the native adapter owns document
preparation and typed native results, and the PoB adapter owns optional Lua reference hosting.
Neither the native engine nor future browser bindings should depend on the native PoB
worker package.

The long-term replacement is a native evaluator behind the same semantic evaluation
contract: actor/skill/scenario selection, metric units, coverage declarations and data
identity must remain comparable. A partially translated engine must report its supported
scope. Do not fill unsupported metrics with zero or silently combine native and Lua
outputs from different candidate states. A deliberate hybrid path needs explicit ownership
of each calculation stage and parity at its boundaries.

Pure calculations neither allocate a thread pool nor perform I/O. The native desktop
runner can schedule independent candidate/scenario evaluations through the shared Rayon
budget. A browser runner supplies its own scheduling, progress, cancellation and data
loading while calling the same Rust calculations. Persistent mutable caches, when added,
belong to one evaluation context and must demonstrate request-order independence.

## First translation boundary: defence kernels

The initial native surface is small enough to compare directly with upstream code, and
useful inside a later defence pipeline. All outputs below are percentage points rather
than 0-1 fractions; rating and raw hit inputs are already resolved by the caller.

| Native function | Pinned upstream function | Contract |
| --- | --- | --- |
| `hit_chance` | `calcs.hitChance` | Player hit formula with a 5% floor and optional removal of the 100% ceiling. Negative accuracy returns 5 immediately. |
| `monster_hit_chance` | `calcs.monsterHitChance` | Separate monster hit formula, rounded and clamped to 5-100%. |
| `deflect_chance` | `calcs.deflectChance` | Deflection formula with the selected data cap; rating below 1 returns 0. |
| `armour_reduction_percent` | `calcs.armourReductionF` | Fractional percent reduction for one raw hit, including negative-armour behavior. Later mitigation caps are outside this helper. |
| `armour_reduction_rounded_percent` | `calcs.armourReduction` | The same reduction with upstream integer rounding. |
| `round_to_integer` | `round`, without `dec` | `floor(value + 0.5)`, including negative half-integers. |

Source is [CalcDefence.lua:33-69](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Modules/CalcDefence.lua#L33-L69)
and [Common.lua:722-728](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Modules/Common.lua#L722-L728).
`DefenceConstants` keeps data inputs explicit. The pinned defaults are armour ratio 10
and deflection cap 95 from [Modules/Data.lua](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Modules/Data.lua#L251-L261).
The upstream MIT attribution is retained in the crate's
[NOTICE.md](../crates/poe-optimizer-engine/NOTICE.md).

These helpers do not collect modifiers, apply item/passive rules, compute complete EHP,
or evaluate a build. Their constants are passed explicitly rather than being a second
independent game-data database.

## Second translation boundary: numeric ModDB queries

`modifiers::ModifierDatabase` is an immutable numeric aggregation slice with explicit
`QueryContext` and `MorePrecision` inputs. It models the pinned `ModDB` methods, not the
entire modifier store and not `ModList` interchangeability. A `ModifierInput` preserves
its source, modifier/value kinds and tag names until validation. Construction rejects the
entire input if any layer contains an unsupported entry, even if a particular query
would not select that entry. This keeps partial extraction from silently changing a build.
No automatic Lua extraction or full-build adapter is attached to this slice yet.

| Surface | Supported semantics |
| --- | --- |
| `sum(Base / Increased)` | BASE/INC addition by query-name then insertion order; each local result adds its recursively grouped parent result. |
| `more` | MORE percentages become multiplicative factors. Default rounding occurs per local stat bucket at two decimal places; explicit high precision truncates the accumulated result. Precision carries across query names within a layer and resets for each parent. |
| `override_value` | First matching local value in name/insertion order, then parent layers. Zero is a present override; absence returns `None`. |
| Modifier flags | All required bits must occur in the query. Exactly representable nonnegative 53-bit masks are supported except bit 31, whose upstream signed-low-word behavior requires a separate extension. Every currently declared pinned `ModFlag` fits the supported domain. |
| Keyword flags | Any matching keyword by default, all keywords when the modifier carries `MatchAll`. Empty requirements match. The `MatchAll` control bit is removed from both masks before matching; masks are bounded to bits 0-30. |
| Source provenance | Strings remain attached to modifiers. BASE/INC accept the exact source or its first nonempty colon-delimited component. MORE/OVERRIDE match only that component. A selected modifier with absent source and a source-filtered MORE/OVERRIDE query produces an explicit error, matching the upstream error boundary. |
| Parent layers | Layer zero is the queried DB and following layers are its successive parents. Layer and insertion order are semantic inputs, retained for rounding, floating-point cancellation and override priority. |
| Query names | Zero through eight names, preserving order and repeated names. More than eight is rejected. |

`MorePrecision::pinned()` contains the two MORE entries from pinned `Modules/Data.lua`:
`SupportManaMultiplier` and `ReservationMultiplier`, both at four decimal places.
`try_new` accepts an explicit alternate precision table with decimal places 0-15. The
empty/default table means ordinary two-decimal rounding everywhere; it must not be
mistaken for the pinned game data. Precision inputs concern MORE aggregation only;
modifier scaling has separate rules implemented by the bounded program below.

The legacy `try_new` constructor still rejects every raw tag name. The explicit typed
condition path below handles a declared subset of `EvalMod`; item, skill, multiplier,
threshold, global-limit and other tags remain unsupported. FLAG/LIST/MAX and unknown
modifier kinds, nonnumeric values including functions/tables/booleans/nil, unsupported
flag masks and out-of-range precision fail explicitly. A future importer must carry
unsupported metadata to validation rather than constructing an untagged approximation.
This module does not derive a final stat by assuming a universal combination of BASE,
INC, MORE and OVERRIDE.

The independent differential harness executes the actual pinned `ModStore` public query
wrappers and `ModDB` implementation. It loads the actual `Data/Global.lua` bit/keyword
helpers and extracts the actual `Common.lua` class library/rounding and `Data.lua` precision
table. The five calculation/helper source hashes and the actual ModTools constructor source hash are checked. There is no translated Lua
formula oracle. Cases cover interpreted and warmed LuaJIT, interacting flag/keyword masks,
53-bit boundaries, parent layers, duplicate and reordered stat names, source variations,
zero overrides, precision carry, negative and near-rounding-boundary factors, grouped
cancellation, nonfinite values and fail-closed unsupported input. Finite comparisons use
`1e-12 * max(1, abs(reference))`; signed zero/infinity and NaN classification are checked
separately. Captured real modifier contexts and whole-build mutation parity remain gates
before native evaluator integration.

Sources: [ModDB.lua](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Classes/ModDB.lua),
[ModStore.lua](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Classes/ModStore.lua),
[Global.lua](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Data/Global.lua#L122-L332),
[MORE precision data](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Modules/Data.lua#L597-L603).

## Third translation boundary: explicit conditional modifier contexts

`ModifierDatabase::try_new_tagged` accepts `TaggedModifierInput` without changing the
legacy untagged constructor or query signatures. Each input keeps its complete numeric
modifier, source, flags and an ordered vector of typed `ModifierTag` values. Unknown tags
or unrepresented fields must become `Unsupported` metadata and reject the complete input,
including entries no current query would select. Tagged databases require the explicit
`sum_with_conditions`, `more_with_conditions` or `override_with_conditions` query path;
calling an old query on them returns `MissingConditionContext`.

`ConditionEnvironment::try_new` validates an immutable environment. It contains the
queried store's local/parent condition tables, an actor index graph, per-actor condition
tables and weapon metadata, and the query's override/skill conditions and actor name.
References must resolve; every store condition layer corresponds to a modifier layer.
An absent actor link is distinct from an invalid link. Contexts retain unsupported-feature
markers so extraction cannot discard condition-producing modifiers or unknown behavior.
They do not own Lua objects, mutable caches, a thread pool or operating-system services.

| Supported behavior | Pinned semantics |
| --- | --- |
| `Condition` with one variable or a list | A list is OR, ordered tags are AND; negation applies to the complete tag. An empty list is false before negation. |
| `GetCondition` table lookup | A present `overrideCond` value wins, including false. Otherwise local or any parent true is sufficient; local false does not mask a true parent. |
| Skill-local conditions | `Condition` checks `skillCond` after `GetCondition`, so a true skill condition can satisfy even a false override. `ActorCondition` ignores `skillCond`. |
| `ActorCondition` | Supports an explicit actor or the current queried store, optional one/list variables, negation, and the upstream `cfg.actor` fallback when no condition target is present. Enemy conditions use this tag with `actor = "enemy"`; the pin has no `EnemyCondition` tag. |
| Player actor lookup | Direct player reference first, then the parent actor's player, then the enemy actor's player. Other roles use direct links. Parent actor references and parent modifier layers are separate inputs. |
| Parent modifier evaluation | A modifier inherited from a parent DB still evaluates against the original queried store and actor context, preserving the `context` argument threaded through `ModDB`. |
| All-one-handed weapon exception | Negated `Condition` tags retain `countsAsAll1H` and `Added<condition>` behavior, including first qualifying weapon precedence and list order. |
| Disabled conditional values | BASE/INC contribute zero; MORE uses zero but still influences precision selection; OVERRIDE is absent. Active numeric zero remains a present override. Source errors occur before tag evaluation. |

This is an **explicit condition-table subset**. `GetCondition` also consults
`Condition:<name>` FLAG modifiers upstream. FLAG inputs still reject at numeric DB
construction, and condition-producing FLAG entries in actor/context extraction must be
retained as unsupported features. They must not be pre-resolved into apparently complete
booleans unless a separate extraction contract proves equivalence across the exact query
flags, source and overrides. No real-build modifier extractor is provided yet.

The multiplier slice below handles explicit values and conditional numeric producers.
Recursive multiplier-producing tags, global limits, item/skill predicates, modifier
functions and condition-producing FLAG queries still need their own typed input and
parity scope before activation.

The differential suite uses the same source-hashed, actual upstream `ModStore`/`ModDB`
harness as numeric aggregation; it does not replace `EvalMod` or `GetCondition` with a
copied oracle. It exercises interpreted and warmed LuaJIT across truthy inheritance,
false overrides, skill-local conditions, missing actors, player fallback precedence,
weapon exceptions, source and flag filtering, inactive MORE precision, zero overrides,
nonfinite values and explicit rejection. Captured real modifier contexts and complete
candidate mutation parity remain required before integrating this slice into a native
build evaluator. No throughput improvement is claimed.

## Fourth translation boundary: explicit multipliers and numeric scaling

`multipliers::MultiplierEnvironment` combines the complete current store's explicit
multiplier tables with the validated numeric/conditional `ModifierDatabase` and
`ConditionEnvironment`. All three retain the same nonempty local/parent layer structure.
Unsupported context metadata rejects construction. `get_multiplier` executes pinned
`GetMultiplier`: a present OVERRIDE wins (including zero or NaN); otherwise the local
explicit value adds the recursively grouped parent explicit values, then the complete
BASE modifier query. Parent BASE/OVERRIDE queries are not repeated while traversing
explicit values. Flags, source filters, conditions and override errors use the existing
actual-source-tested query semantics.

`ScalingProgram` validates and evaluates a complete ordered sequence of numeric tags.
It returns `None` for a disabled modifier and `Some(0)` for an active numeric zero. It is
an `EvalMod` slice, not a replacement for ModDB query selection or aggregation. Callers
remain responsible for the surrounding query's modifier-kind, flags and source checks.
No full-build extraction, native evaluator integration or throughput claim follows.

| Surface | Supported semantics |
| --- | --- |
| `Multiplier` | One variable or a dense ordered array sum, literal divisor or current-store `divVar`, `floor(base / divisor + 0.0001)`, optional inversion of a nonzero factor, and additive `tag.base` after multiplication. |
| Multiplier caps | Literal/current-store multiplier cap applied either to the factor, the resulting total maximum, or the resulting total minimum. Factor caps apply before inversion; total caps apply after multiplication and additive base. |
| `MultiplierThreshold` | Literal/current-store multiplier threshold; exact upper/lower/equality comparisons and the upstream interaction when both `upper` and `equals` are true. NaN comparisons retain upstream behavior. |
| `Limit` | Literal/current-store multiplier ceiling, or a floor at the negated limit. Equal and unordered min/max operands preserve the selected x64 LuaJIT behavior. |
| Ordered condition tags | The already supported `Condition` and `ActorCondition` predicates can appear among scaling tags. An early disabled condition prevents later multiplier queries and their errors. |
| Mutable divisor field upstream | `divVar` is read on every evaluation. The native program stores the source reference, so it has no mutable `tag.div` field or request-dependent cache. |

This boundary deliberately excludes multiplier recursion. The environment's numeric DB
can contain BASE/OVERRIDE producers tagged with supported conditions; any nested
multiplier/scaling producer must retain its unsupported tag and fail DB construction.
Actor-specific multiplier/limit/threshold targets also reject, even if the current actor
context could resolve them. ActorCondition remains available solely as a condition gate.
The separate stat slice below adds current-store PerStat and StatThreshold. PercentStat,
reservation-specific GetStat behavior, global/shared limits, mixed-key `varList` tables,
table/function-valued modifiers and unknown fields remain unsupported.
An importer must retain unknown fields as unsupported metadata. For overlapping upstream
fields, it must preserve upstream precedence: `divVar` over `div`, a literal cap over
`limitVar`, and `limitTotal` over `limitNegTotal`. The typed cap mode records the selected
meaning; it does not infer omitted input semantics.

The differential harness calls the actual pinned `ModStore:GetMultiplier` and
`ModStore:EvalMod` against real ModDB layers. It uses the same calculation/helper normalized full-source
hash checks as the earlier modifier tests. Cases cover both interpreted and warmed LuaJIT,
conditional and source-filtered producers, zero overrides, parent grouping, absent
variables, ordered and repeated variable lists, threshold equality, rounding-boundary
neighbors, zero/negative divisors, inversion and cap order, condition short-circuit errors,
nonfinite arithmetic and signed zero. Production calculation code remains portable and
immutable, with no Lua objects, I/O or host scheduling.

Sources: [GetMultiplier](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Classes/ModStore.lua#L417-L423),
[Multiplier and MultiplierThreshold](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Classes/ModStore.lua#L489-L604),
[Limit](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Classes/ModStore.lua#L739-L741).

## Fifth translation boundary: explicit ordinary stat lookups

`stats::ResolvedStatEnvironment` stores an optional actor output table and the query's
skill-local stat table. Ordinary `GetStat` queries prefer a present output value, including
zero or NaN, then the skill-local value, then zero. An absent output table is preserved.
This context receives resolved numeric values; it does not derive those values from a
build. Unsupported context/value metadata rejects construction.

`ScalingProgram` additionally accepts `PerStat` and `StatThreshold` tags. These programs
require `evaluate_with_stats`; calling the previous `evaluate` method returns
`MissingStatContext`, even when a preceding condition would disable the modifier.
Construction checks every referenced stat and rejects unsupported branches before use.

| Surface | Supported semantics |
| --- | --- |
| `PerStat` | One stat or a dense ordered list, repeated names, literal/current-store multiplier divisor, `floor(stat / divisor + 0.0001)`, additive base, and a literal/current-store multiplier maximum on the factor or final value. |
| `StatThreshold` | One stat or ordered list, literal or named-stat threshold, optional literal/current-store multiplier percentage, and exact upper/lower comparisons. An absent percentage skips that arithmetic; zero remains a present percentage. |
| Ordered composition | Stat tags execute among existing multiplier, limit and condition tags, preserving early exits and numeric operation ordering. |
| Dynamic inputs | Stats and divisor references are re-read from the supplied immutable contexts on each call. Programs retain no request-dependent values. |

`ManaReservedPercent`, `LifeReservedPercent` and `ManaUnreserved` are explicitly rejected
as queries. Upstream treats these names specially using reservation/skill data or a NaN
fallback; a plain numeric table lookup is not equivalent. These keys may remain in an
output table, but neither direct queries nor program references silently approximate
them. Actor-targeted PerStat, PercentStat, recursive scaling producers, function/table
values, mixed-key stat lists and unknown fields remain outside this slice. The importer
must retain unrepresented fields as unsupported metadata.

The actual-source tests call `ModStore:GetStat` and `EvalMod` in interpreted and warmed
LuaJIT. They exercise output-versus-skill precedence, absent tables and fields, dense list
ordering/cancellation, repeated names, threshold percentage/equality boundaries,
zero/negative divisors, cap order, mixed tag sequences, changed contexts with reused
programs, signed zero, infinities and NaN. No translated Lua formula serves as an oracle.

Sources: [GetStat](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Classes/ModStore.lua#L425-L468),
[PerStat](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Classes/ModStore.lua#L605-L652),
[StatThreshold](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Classes/ModStore.lua#L704-L722).

## Closed native build profiles before general evaluator coverage

The production destination is a fully native build evaluator. PoB supplies optional parity
checks and update evidence; supported native calculations must not launch Lua processes.
Grow complete, explicitly bounded build profiles alongside the common modifier machinery,
so consumers can exercise the whole build/evaluation boundary before the general engine
is ready. Unsupported builds or mechanics must fail explicitly, with the optional reference
backend selected deliberately by the caller rather than hidden inside a native calculation.

The `spark` profile accepts level-one, quality-zero Spark with no equipment or supports.
`evaluate_with_character` receives resolved class attributes and the explicitly admitted
ordinary entrance and ascendancy resistance effects described below. The host validates the complete scope before
constructing `SparkInput` and `CharacterInput`; the numeric engine does not parse XML.
The legacy `evaluate(input)` wrapper preserves Sorceress attributes and no passive effects.
The input exposes character level, resistance penalty, resolved enemy lightning resistance
and six quest reward switches. It returns attributes, life/mana/energy shield, armour,
evasion, four player resistances, selected average hit and hit DPS, plus intermediate cast
rate, critical chance/multiplier and effective enemy resistance. EHP, maximum hits, ailments,
projectile collision/repeat-hit simulation and resource sustainability are not supplied.

The profile preserves PoB's configuration defaults: the six fixed quest rewards affecting
these outputs are enabled unless explicitly disabled, independently of character level.
They add flat life, increased life/mana and elemental resistance. Quest choice lists default
to Nothing. Consequently, a minimal XML with no equipment or allocated nodes still has
quest modifiers; omitting those defaults would produce incorrect resource/resistance values.
Normal, standard and pinnacle enemy resistance defaults are resolved by the host. Uber
boss damage reduction, an explicit `enemyMaxResist` flag and other damage-taken modifiers
remain outside this profile. Enemy resistance uses the pinned configurable ceiling (up to
90%) and floor (-200%); player resistances retain upstream truncation before clamping.

`SparkData` is a compact compiled view of injected skill, class, character, quest and rules
records. `CompiledGameData` resolves the package once; `evaluate_with_data` borrows it.
See [native data packages](native-data.md) for the loader and compatibility wrappers. `SOURCE_FILES` identifies
12 complete normalized source files, including the modifier parser semantic oracle.
`PROFILE_ID` is `poe2-spark-level1-class-passives-v3`.
The production function uses no parsing, allocation, I/O, timing, Lua or shared state. The
application adapter owns XML admission, source identity, evaluation clock, metric coverage
and prepared-input reuse. A native-only build and WASM consumer can therefore call the
same function without the PoB package.

Validation combines two complementary forms of evidence. The source harness executes the
actual ModDB/ModStore machinery, complete resource function and unchanged relevant offence
expressions for the admitted profile across levels, quest combinations and resistance
boundaries, in interpreted and warmed LuaJIT. Full-build checks compare the native outputs
against the immutable independent mapping/pinnacle Spark goldens; new host integration
fixtures must also compare complete admitted inputs with the optional PoB backend. The
production calculation never reads golden measurements. Native-only test dependencies load
reference JSON and Lua; they are excluded from the production/WASM dependency graph.

A closed-profile comparison proves only its declared scope. Expand admission and tests
together when adding gem levels, supports, equipment, passive paths, classes or conditions.
Preserve exact candidate realization checks and metric availability at each expansion;
never let a broad parser make unsupported mechanics disappear before profile admission.

Sources: [Spark skill data](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Data/Skills/act_int.lua#L19901-L20124),
[base initialization](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Modules/CalcSetup.lua#L826-L849),
[attribute bonuses](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Modules/CalcPerform.lua#L489-L520),
[quest defaults](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Modules/ConfigOptions.lua#L57-L105),
[resource calculation](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Modules/CalcDefence.lua#L71-L128),
[hit/DPS aggregation](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Modules/CalcOffence.lua#L4563-L4576).

### Closed Mace Strike profile

The `mace` profile accepts level-one Mace Strike, one normal Wooden Club or Smithing
Hammer, integer quality 0..20 and item level 1..100. Zero to two level-one quality-zero supports from
Brutality I, Heavy Swing and Rapid Attacks I are admitted. See [support composition](support-loadouts.md). `evaluate_with_character` adds resolved class attributes
and the ordinary entrance/ascendancy resistance effects below. There are no other items,
supports, ascendancy effects or external modifiers. The legacy `evaluate(input)` wrapper preserves
Warrior attributes and no passive effects. Warrior is entry 3 in the raw tree classes array;
its `integerId` and PoB's rekeyed live class index are 6. The host validates the complete
document, identities and scenario before constructing `MaceInput` and `CharacterInput`.
Unmodified weapon base stats do not scale with item level.

The production kernel translates local weapon quality and endpoint rounding, the skill's
base damage and Brutality's modifier/elemental damage exclusion, inherent accuracy,
rounded enemy evasion, hit chance, a second accuracy check for critical strikes, attack
speed, separate critical/ordinary armour mitigation and final hit DPS. The enemy physical
mitigation cap is 75%, separately sourced from the player cap. Applying one armour
reduction to the combined average would be incorrect because a critical strike's larger
hit has a different reduction. Character resources and resistances use the same quest and
character-data source pipeline as Spark, with the explicitly resolved attributes.

`MaceInput` receives resolved finite, nonnegative enemy armour/evasion and finite fire
resistance in -200..200. The kernel applies the ordinary configurable resistance cap and
provides checked source-table lookups for normal-monster levels 1..100. The adapter must
apply actual encounter level bounds and defaults; the initial document profile admits
normal enemies and explicit armour. Other encounters, enemy block, nondefault distance,
armour break, physical reduction modifiers, damage conversion, lucky damage, ailments,
additional support mechanics and other unrepresented flags require new parity-backed
coverage. Per-hand average hit is exposed as `main_hand_average_hit`; PoB does not provide
a top-level AverageHit for this attack, and the typed metric remains unavailable under
that existing contract. This profile claims hit DPS, not combined or ailment DPS.

Twenty-one normalized source hashes accompany the Rust data; the profile identity is
`poe2-mace-strike-support-loadouts-v4`. The differential test executes
actual pinned Item/ModDB/resource/offence source with resolved closed-profile scaffolding,
including the real Brutality stat map and damage-disable flags. Interpreted and warmed
runs cover every admitted quality, both weapons and support choices, character levels,
armour caps, evasion rounding and resistance boundaries, quest toggles, nonfinite input
rejection and repeated requests. Four separately captured, unchanged full-build attack
goldens provide an additional check. Full document/mutation comparisons remain necessary
before an adapter expands its accepted scope. These tests establish parity for this
closed profile and do not establish a general native build engine or game certification.

### Resolved class attributes and ordinary entrance effects

`character::CharacterInput` is shared by both complete kernels. Its `attributes` field
contains resolved strength, dexterity and intelligence. These are whole nonnegative values;
class catalog lookup, class-specific node replacements and build admission live in the
portable data/native adapter. The calculation remains independent of XML, the data loader,
Lua, operating-system services and mutable caches. Every class in the pinned catalog can
supply these values. An ascendancy identity with no allocated ascendancy effects does not
add numeric modifiers to these profiles.

`CharacterModifiers` admits the numeric forms present on the two ordinary entrance
nodes for each class plus five signed BASE resistance operations. It is not a general
modifier parser:

| Explicit fields | Closed-profile effect |
| --- | --- |
| `armour_flat`, `evasion_flat`, `energy_shield_flat` | Global defence bases with upstream final integer rounding. Base evasion is 7; armour and energy shield otherwise begin at zero. |
| `skill_speed_increased` | Applies to each profile's attack/cast speed; the total multiplier rounds to two decimals before dividing base time. The source also creates warcry/totem-placement speed modifiers, which have no action target in these profiles. |
| `spell_damage_increased`, `projectile_damage_increased` | Both match Spark and add before each lightning damage endpoint is rounded. They do not match Mace Strike. |
| `attack_damage_increased`, `melee_damage_increased` | Both match Mace Strike and add before damage scaling by Brutality and endpoint rounding. The physical weapon endpoints have already passed local quality rounding. They do not match Spark. |
| `fire_resistance_flat`, `cold_resistance_flat`, `lightning_resistance_flat`, `chaos_resistance_flat`, `elemental_resistance_flat` | Signed BASE player contributions, using the shared resistance calculation described below. |
| `minion_damage_increased` | Preserves the Witch entrance's explicitly scoped minion bonus. Neither zero-minion profile has an actor to receive it; it does not increase player damage. |

The Witch replacement at physical node 4739 has spell and minion damage; the Huntress
replacement at physical node 56651 has attack damage. Their shared-start counterparts
have spell and projectile damage respectively. Druid node 50084 has both spell and attack
damage; only the matching flag contributes to either selected skill. Resolve these source
replacements before constructing numeric inputs. Do not sum the original and replacement
node effects or use the effective replacement ID as a physical graph allocation.

Attributes affect life, mana and accuracy through the actual inherent bonuses. These
entrances introduce no flat/increased life, mana, accuracy or attribute modifiers, so the
input does not imply support for those wider modifier forms. Nonfinite, negative or
fractional attributes reject. Old modifier fields must be finite and nonnegative; the five
resistance fields admit signed finite values. The numeric
boundary caps every field at 1,000,000 to keep admitted calculations finite; this is an
implementation scope bound, not a game stat maximum. The adapter restricts operations to
the currently supported effects and zero or one ordinary entrance plus zero or one
admitted ascendancy passive. Magnitudes
come from the selected validated package; only the reviewed default has source-parity
evidence. Diagnostic
evaluation does not certify available passive points or skill/item attribute requirements;
search must enforce its explicit progression and legality constraints separately.

The source oracle loads the full pinned `ModParser.lua`, checking its complete normalized
hash, and proves the exact numeric names, flags and nested minion scope for all sixteen
effective entrances. No general text parser is implemented in Rust. Both kernel oracles
execute the original `calcDamage`, resource/attribute and global defence branches, in
interpreted and warmed LuaJIT. Class attributes and effective stat strings are read from
the actual pinned tree. Cases cover every entrance across character levels and mitigation,
plus damage, speed and defence rounding boundaries, rejected exceptional inputs and request
reuse. The six immutable independent Spark/Mace calibration builds remain unchanged.
Complete document comparisons are a separate integration gate; these calculations do not
establish general tree, ascendancy-mechanic or game legality coverage.

Sources: [ordinary stat parser](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Modules/ModParser.lua),
[pinned class and passive data](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/TreeData/0_5/tree.lua),
[global defence calculation](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Modules/CalcDefence.lua#L1344-L1458),
[damage endpoint calculation](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Modules/CalcOffence.lua#L178-L225),
[speed multiplier rounding](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Modules/CalcOffence.lua#L2975-L2980).

## Numeric behavior and parity

Use `f64` with the original operation ordering and no fast-math reassociation. Upstream
rounding differs from Rust's `f64::round()` at negative half-integers. Negative armour uses
its positive magnitude in the denominator and negates the result; its rounded form need
not be symmetric at a half-integer. Preserve these details until a separately reviewed
game-mechanics correction intentionally changes the contract.

Low-level kernels retain upstream exceptional arithmetic. The explicit zero-armour,
zero-hit case returns zero; singular denominators and nonfinite inputs can otherwise
produce infinities or NaN. Upstream x64 LuaJIT ordered min/max clamps can turn an intermediate
NaN into a finite boundary. Native comparisons model that ordering explicitly rather than
relying on Rust or WASM min/max NaN behavior. These semantics are a compatibility contract,
not proof that such an input is a valid game state. Candidate validation must reject invalid
inputs independently. Keep NaN, positive infinity and negative infinity distinct in metric
serialization and feasibility policies. NaN payload bits have no semantic guarantee.

Each translated slice must pass:

1. A source identity check tied to the submodule revision and normalized source hashes.
2. Differential tests executing the actual upstream functions, with the minimum necessary
   environment and data. A copy of the Rust formula rewritten in Lua is not an oracle.
3. Boundary cases, branch/override cases, singular arithmetic and relevant property checks.
4. Differential comparisons in interpreted and warmed LuaJIT execution where applicable.
5. Integration comparisons using real candidate states before activating the slice inside
   the native build evaluator.

The kernel differential harness loads the actual pinned defence module with a minimal
`Modules.CalcBase` table, and extracts the actual upstream `round` definition. It checks
CRLF-normalized SHA-256 hashes for the defence, common and data modules; changing the pin
requires an explicit translation review. Checks include rating grids, half-integer and
percentage-threshold neighbours, early returns, negative armour, alternate constants,
positive-domain monotonicity, nonfinite classifications and signed zero. Finite fractional
results use a declared `1e-11 * max(1, abs(reference))` tolerance; infinities retain their
signs and NaN is compared by classification. No performance claim follows from these tests.

The Rust production library has no external dependencies. Lua and source hashing are
native-only development dependencies used by the test oracle. Maintain this separation
as the native evaluator expands; shared data types must not introduce a transitive Lua
runtime dependency.

## Expansion order and hard boundaries

The native track can advance in parallel with evaluator hardening and search development.
Prioritize cohesive stages with explicit inputs/outputs over arbitrary lines of Lua:

| Stage | Boundary to preserve | Main parity evidence |
| --- | --- | --- |
| Numeric kernels | Units, rounding, constants, exceptional arithmetic | Direct differential grids and branch cases |
| Modifier aggregation | BASE/INC/MORE/OVERRIDE, flags, tags, scopes, conditions and source provenance | Small exhaustive modifier sets and captured real modifier contexts |
| Resource and resistance pipeline | Equipment conversions, reservation, maximum/current pools, caps, overrides and actor inheritance | Complete intermediate outputs and controlled candidate mutations |
| Active skill/minion pipeline | Skill/stat-set identities, support compatibility, granted effects, triggers, actor and action selection | Cross-skill/support interactions, exact selections and fixture deltas |
| Full defence/offence pipeline | Damage conversions, ailments, costs, recovery, avoidance, maximum hits and encounter assumptions | Independent build/scenario parity across representative mechanics |
| Native evaluator integration | One complete candidate, versioned data, coverage declaration, export semantics | Full contract parity and search-result re-evaluation |

Modifier DB semantics and normalized game data are substantial prerequisites. PoB uses
mutable global tables, modifier functions/closures, private classes, conditional tags and
version-specific preprocessing; translating the arithmetic alone does not remove them.
Design a typed, validated, versioned data snapshot with stable IDs, source hashes and
unsupported-field reporting. Extraction can initially use the reference adapter as a
build-time tool; the deployed native/browser evaluator must consume data without Lua.
Separate extraction-tool requirements from runtime requirements and retain applicable
upstream notices with generated/derived data.

Keep the reference adapter after a native path becomes useful. It supplies differential
checks for upstream updates and mechanisms not yet translated. A new game patch changes
both calculations and data; record capabilities by game/data/adapter identity and rerun
affected parity fixtures rather than silently comparing incompatible revisions.

## WebAssembly deployment

Target `wasm32-unknown-unknown` for the portable calculation library. The kernel dependency
graph contains no Lua/C runtime or OS calls. Compilation is only the first gate: browser
bindings, downloadable data, input limits, cancellation, memory budgets and execution tests
are additional work. The Rust target supports `core`/`alloc` and portions of `std`, while
filesystem and thread APIs do not provide normal native behavior. Keep those services in
the host. [Rust target documentation](https://doc.rust-lang.org/rustc/platform-support/wasm32-unknown-unknown.html)

Start browser execution in a Web Worker, with a portable sequential path and bounded work
batches. Future parallel execution may use several independent workers or shared-memory
Rayon support. Rayon ordinarily falls back to sequential operation for WebAssembly without
thread support. [Rayon documentation](https://github.com/rayon-rs/rayon#usage-with-webassembly)
Shared-memory browser Rayon requires an adapter, worker initialization and cross-origin
isolation for SharedArrayBuffer; check the deployment environment before committing to
that mode. [wasm-bindgen-rayon documentation](https://github.com/RReverser/wasm-bindgen-rayon#setting-up)

Keep browser scheduling outside the math crate so desktop multicore performance and
browser compatibility can evolve independently. Feature-detect the selected browser
execution mode and record it in results. Do not promise equivalent throughput, memory
capacity, or wall-clock determinism across native and browser hosts.

The portable compilation gate is:

```powershell
cargo check -p poe-optimizer-engine --lib --target wasm32-unknown-unknown --locked
```

Native differential validation is:

```powershell
cargo test -p poe-optimizer-engine --locked
```

Record actual command results and unverified targets in the living implementation log.

## Shared player resistance calculation

The admitted Spark and Mace pipelines use `resistance.rs` with injected BASE contributions,
quest rewards, penalty and defence limits. Five typed flat operations cover fire, cold,
lightning, chaos and elemental resistance. Only the new resistance operations admit negative
values; old modifier fields retain nonnegative bounds. Ordinary and ascendancy contributions
combine through `CharacterModifiers::checked_add` and retain explicit ownership in the
compiled lookup. Text matching occurs only in reviewed offline extraction.

For each elemental type the kernel sums that type, penalty and its quest reward, then
adds the elemental bucket. Chaos uses only its own bucket. It truncates totals and limits
toward zero, caps the configured base maximum by injected `resistance_maximum_cap`, then
applies the floor. This matches the pinned `CalcDefence` branch, including negative totals
and fractional custom limits. Earlier custom data could retain fractional limits or exceed
the global maximum; these cases now follow source. Reviewed defaults and goldens are stable.

Original parser and defence-branch tests execute cold and warmed Lua with signed/fractional
values and cap/floor boundaries. Fresh complete-build comparisons cover the four owned
ascendancy nodes, ordinary entrance/support interactions, effect removal and export reloads.
Maximum-resistance modifiers, conditions, INC/MORE/OVERRIDE, conversions and other actors
remain unsupported. Successful validation is scoped to these admitted records.
