# Intrinsic added-damage domain review

Scope: no imported party effects and fully accounted source inputs; pinned PoB `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`, exact
SummonSkeletalSnipersPlayer / RaisedSkeletonSniper / MinionMeleeBow intrinsic source.
All source files are authenticated by the manifest. This is offline source
correspondence, not game-law authority for every minion or every numeric input.

## Constructed data and normal input

The original Data module constructs every skill, support and global stat map using its
original functions and module order. The existing recursive inventory follows nested
modifier tables and records aliases/metatables; selected callbacks are inspected
independently of alias traversal. The original ModCache table is evaluated in the same
bounded host and added to that inventory. Modifier records named AddedDamage or
AddedPhysicalDamage are retained with their entire raw record, including zero values,
tags and nested scope. The only constructed candidates are the two global MORE recipes
at SkillStatMap 1016–1021. A separate complete skill/support quality,
alternate-quality, stat-set, constant-stat census finds no consumers of either key.
ExtraSkillStat records and literal-key factories are separately inspected because they
can deliver mapped stats after ordinary construction. No such delivery names either key.
A changed actual constructor producing INC, zero MORE, conditional/nested/dynamically
spelled AddedDamage or AddedPhysicalDamage is refused by semantic tests even when its
source hash is recomputed.

Normal item, passive and configuration text passes through the original
ModParser/ModCache machinery. Flat physical item-affix identifiers are not the
AddedPhysicalDamage scaling channel. The literal-reference ledger includes all exact
AddedDamage/AddedPhysicalDamage, Added concatenation fragments and the two mapped-stat
keys across pinned Modules, Classes and Data. The only live scaling writer is
CalcActiveSkill 709. ModStore 747/750/759/760 uses Added-prefixed condition names during
predicate evaluation; it does not create scaling modifiers. CalcOffence 4134 constructs
the channel name to read it. Source file pins cover the entire parser, config and
transport implementations, not just matching lines. New code paths or callback bodies
require re-review; this is not a general Lua static analyzer.

## Selected original writer and arithmetic

CalcActiveSkill 703–710 first checks the exact minion profile. Presence of damageFixup
chooses another branch, including a present zero. The authenticated complete profile has
no damageFixup, damage 1.15 and the exact source identity. The intrinsic writer adds raw
MORE `(damage - 1) * 100`, with Attack flags and a negative Spectre/Companion
summon-skill filter. Existing retained full-build observations preserve the actual raw
14.999999999999991 record, applicable source/recipient configuration and final combined
1.15. The current native ActorSlot 001f fixed source program produces profile 2537=1.15
from injected configuration data, then the retained percentage and Action factor
programs preserve that operation sequence.

CalcTools 16–35 combines one INC sum with one multi-name MORE result. CalcOffence 4134
requests AddedPhysicalDamage followed by AddedDamage. ModList 196–229 and ModDB
254–294 reduce each name locally, round the local product, then multiply
inherited-store results. The precision variable can carry across names, so the entire
original highPrecisionMods table is retained and neither relevant channel may appear.
The first name has no supplier; the second has only the intrinsic local record, with no
inherited or INC suppliers in the reviewed no-imported-party domain. Thus the exact
local-group product/round operation yields the complete combined factor for this source.
It is not valid to replace arbitrary multi-name/multi-store grouping with a generic
Product.

The native query supplies only the authenticated individual factor. Its bounded product
is 1.15 (or the reviewed empty identity 1), so the source multiplication/floor/division
steps are finite and do not depend on an unapproved universal numeric-domain policy. All
full-build, unrelated modifier, flat-damage, routing and support ownership boundaries
remain explicit.

The ledger also retains four descriptive occurrences: gem_stat_descriptions.lua
6013/33012 and skill_stat_descriptions.lua 9193/40563. Each pair is the stat key in
an original display-description record and its reverse index. These records render
text; they neither construct a modifier nor add a consuming skill/support stat.
Their presence does not authorize the dormant mechanics recipes.

## Excluded saved party suppliers and admission boundary

`PartyTab.Load` (516-543) sends saved `ImportedBuffs` text to `ParseBuffs`. The latter
can invoke `ModTools.parseFormattedSourceMod` (123-141), which accepts numeric
modifier names/types directly. Aura/otherEffects records can reach minion modifier
stores through CalcPerform 2798-2833 and 3127 onward; EnemyMods is also added by
CalcSetup 912. These are genuine normal saved-input suppliers, including AddedDamage
and AddedPhysicalDamage, and are not bounded by the catalogue census above.

Consequently this proof requires accounted absence of imported-party payloads. The
ordinary fixture precondition accepts only absent or a unique plain Party leaf with
reviewed UI attributes; it rejects children, text, duplicate/namespaced/unknown fields
and malformed booleans. It runs before finite fixture completion. Current owned fresh
normalization retains unsupported source rows under its existing selected fallback
obligations; actual normalization plus check-owned-draft controls must preserve these
links and report Pending for empty, zero and signed formatted payloads. No production
Party disposition, generic raw-input rejection or new native model is introduced.
The global and relevant owners remain Partial. Original Party source row 796 links
to selected choice issue 01f2 (configuration-roles-not-converted). All four original
issues remain, including the separate external-assumptions issue 0207. Mutating the
XML also preserves the two exact saved Action source-snapshot refusal guards; they
are additional obligations, not replacements for the party input obligation.
Native direct builds have only their declared owned contributors; a new external
contribution model must supply reviewed membership before admission.
