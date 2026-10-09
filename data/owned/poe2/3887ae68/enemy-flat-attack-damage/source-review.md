# Enemy flat physical source domain

This offline proof admits an empty `SelfPhysicalMin` / `SelfPhysicalMax` source
domain for the already reviewed intrinsic Skeletal Sniper Basic Attack at PoB
revision `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`. It requires accounted source
inputs and absence of imported Party payloads. It does not admit hostile minions,
other source selections, arbitrary serialized modifiers or future source data.
It closes no owned definition, query or build obligation and allocates no IDs.

## Original construction and retained observations

The acquisition reuses the original source-selection Lua host and executes the
original skills, shared stat mappings, minions, ModCache, BossSkills and ModMap
constructors. It traverses every raw nested table, including inactive, zero and
unselected modifiers. Modifiers are recognized independently of their numeric
value, type or eligibility; nested `EnemyModifier` records are included. Known
aliases and metatables retain the previous checked policy. All root tables stay
strongly referenced while the traversal retains pointer identities, including
across a forced collection before loading ModCache.

The original skills/minions census has 112,987 tables and 344,790 rows. ModCache
extends this to 145,363 tables, 436,112 rows and 11,851 modifier records. BossSkills
and ModMap extend the traversal to 145,688 tables and 436,948 rows, with the same
11,851 modifiers. There are 31 distinct `Self`-prefixed modifier names and 57
callback records (16 catalogue callbacks plus 41 ModMap callbacks). Neither
target name occurs. Seven BossSkills additional-stat maps contain only the
recorded avoidance flags and physical-damage conversion fields. The checker
binds this complete expanded census and those maps exactly, not just their sizes.

The packet depends on the exact existing selection and combined AddedDamage
proofs, including their complete pinned Lua file map. This avoids duplicating
that large manifest and profile evidence. It also authenticates the complete
retained preconversion report: original Physical base calls have an empty raw
Enemy inventory and both original eligible BASE queries return positive zero.
The exact Actor store, Enemy store, summoner, parent, pass configuration and
selected source identities are checked. Unexecuted CALCS calls remain
unobserved; they do not establish an empty result.

The complete selected profile has no `hostile` field. `CalcActiveSkill.lua`
939–947 therefore sets its enemy to `env.enemy`; hostile minions would instead
target the Player and are outside this proof. `CalcOffence.lua` 529 takes
`actor.enemy.modDB`, 3440 selects `pass.cfg`, and 4132–4133 query the two exact
Enemy names with that configuration. Profile name or one observed zero alone
does not establish this domain.

## Factory and transport review

The token ledger conservatively retains 377 computed first-argument sites, 182
name-field writes and 34 exact `Self`/target string sites across pinned Modules,
Classes and Data. Some `mod(...)` sites are `calcLib.mod` readers; this is a
candidate ledger, not a claim that all 377 sites are writers. Full source hashes
bind the surrounding code and helper bodies. A changed ledger requires review.

* Direct skills, supports, quality mappings, shared stat maps and cached/nested
  records are covered by the original constructed traversal. No matching record
  exists, even with a zero value or an inactive condition.
* `ConfigOptions.lua` 2250–2256 is a real arbitrary-key writer from
  `bossSkills[val].additionalStats.base/uber`. The actual constructed maps are
  retained and checked; this transport is not dismissed as a fixed-name writer.
* `Data.lua` 730 loads `ModMap.lua`. Its 41 constructed callback source spans
  have fixed stat names, including `SelfCritMultiplier`, and no target name.
  No active consumer beyond that load was found in the pinned Modules/Classes;
  this is a reviewed dormant library, not newly admitted configuration behavior.
* Generic ModTools constructors and ModStore/ModDB/ModList additions preserve
  their supplied names. EnemyModifier, curse, debuff, ailment and buff transports
  preserve names from the reviewed records. ExtraCurse resolves known skills and
  their curse effects. The ailment callbacks at `CalcPerform.lua` 3250–3290 write
  ActionSpeed, ColdDamageTaken, DamageTaken and DamageTakenByShock, not flat
  physical damage received by the Enemy.
* Runtime factories have disjoint name families: resistance and resource
  conversions, recovery/recoup, conditions/multipliers, typed ordinary flat
  damage and leech. `CalcOffence.lua` 500 renames only into typed
  Damage{Life,Mana,EnergyShield}Leech; `CalcPerform.lua` 2344/2717/3199 adds Totem
  prefixes. Mageblood's `entry.stat` at 1574 comes from its fixed local legacy
  table. AbyssalWasting condition strings are restricted to the parser's known
  condition mapping; its other templates have Resist or Instant*Leech names.
* Parser radius-jewel `getSimpleConv`, `getPerStat` and `getThreshold` use the
  reviewed destination families. The one computed simple-conversion destination
  ends in `Damage`. Generic flat factories at `ModParser.lua` 2769–2772 and
  3748–3749 accept `%a+`, not a five-type whitelist. Nevertheless `scan` at
  6596–6606 lowercases captures and `firstToUpper` at 13–15 changes only their
  first letter: `selfPhysical` becomes `Selfphysical`, which cannot construct
  the case-sensitive `SelfPhysicalMin` or `SelfPhysicalMax`. Other dynamic Self
  templates append ailment `Effect` names. This is a source-specific argument,
  not a general parser sanitization claim.
* SkillData indexed writes at `CalcActiveSkill.lua` 896–900 and
  `CalcOffence.lua` 740–742 affect skillData. Weapon/corpse endpoint selection is
  a separate source domain; these paths do not insert Enemy modifier records.

Imported Party data is a genuine counterexample to unconditional absence.
`PartyTab.lua` 516–543 and its ParseBuffs paths preserve formatted records;
`ModTools.lua` 123–141 accepts arbitrary numeric names, including zero and an
empty source string. `CalcSetup.lua` 912 loads EnemyMods directly. Aura,
otherEffects and Warcry transports also deliver imported records to minions.
The existing original-parser control proves that a formatted SelfPhysicalMin
record is accepted. Consequently this packet requires the same accounted,
no-imported-Party source boundary as the combined AddedDamage proof. In the
current Original05 import, Party rows retain selected configuration obligation
01f2; all four selected obligations, including separate external-assumptions
0207, remain unresolved. No SourceOnly classification or completion is inferred.

## Arithmetic consequence and validation boundary

The original ModList and ModDB `SumInternal` implementations begin at positive
zero and add eligible finite values in order, followed by the parent sum. Under
binary64 round-to-nearest addition, this ordered reduction cannot produce
negative zero from that identity: negative nonzero cancellation produces positive
zero, and adding a negative-zero record to positive zero preserves positive zero.
An empty Enemy sum is positive zero. Removing the final `selfSum + enemySum`
addition therefore preserves the bits of an admitted finite ordinary self sum.
This does not authorize an arbitrary independently supplied negative-zero value
or a changed reduction order.

Separately, existing owned `FiniteQuantity::new` canonicalizes either signed
zero to positive zero, and the Engine constructs each numeric node through that
type. Native input and intermediate zero cannot carry a negative sign. Thus the
planned removal of unpublished Enemy ports and their addition needs neither new
zero queries nor a neutral producer. This is an application of existing numeric
semantics, not approval of the pending broader numeric-domain proposal. No
positive endpoint or general floating-point reassociation assumption is needed.

Ordinary tests reconstruct the full original census and compare it with the
retained artifact. Seven source mutations inject actual zero, positive,
conditional, nested, concatenated, loop-generated, minion-profile and BossSkills
target records. Each acquisition recomputes source fingerprints and demonstrates
the real constructed record before semantic refusal; a stale hash is not the
only rejection. Additional artifact controls reject a fabricated zero record,
widened Party/hostile scope, changed Boss maps, altered callback census and a
changed factory ledger even after its local fingerprint is recomputed. Historical
selection, coefficient and AddedDamage reconstruction tests must still pass.
