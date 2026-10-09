# Pinned intrinsic attack-source selection evidence

This packet supports only the source choice for `SummonSkeletalSnipersPlayer` →
`RaisedSkeletonSniper` → `MinionMeleeBow` in PoB revision
`3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`, under normal saved-build import.
It does not certify complete Action routing, final damage, support behavior,
other minions, or complete-build coverage. The native publication separately
checks every owned provider of the exact Basic output. Neither the example's
staff nor its current empty support assignment establishes this invariant.

## Acquisition and admission

`skill_identity_extract::construct_catalog` is the existing bounded original
catalog constructor, now shared privately with this offline evidence test. It
executes original Data/Global, skill definitions, SkillStatMap and gem assembly
through the existing restricted LuaJIT host. It does not replace arithmetic or
execute build calculations. Original Minions.lua is then called with the same
original `mod` and `flag` constructors. Original callback source line positions
are preserved.

The raw graph walk covers every constructed skill, the global stat map, every
constructed minion, nested modifier records, raw stat maps and their metatables.
It retains every constructed SkillData modifier key, every callback source span,
all source-affecting declaration fields, and table aliases. No metatable lookup
or gameplay callback is invoked by the walk. Only the actual original shared
skillStatMapMeta identity is permitted, and only on a statMap table; any other
metatable is rejected, including one on a nested modifier. Selected roots,
stat sets and plain metadata must have no metatable. Direct selected callback
fields are captured independently of graph aliases, so an earlier alias cannot
hide an execution callback. Resource limits and ambiguous key projections fail
acquisition rather than silently dropping records.

The retained JSON also pins every Lua file in the existing source manifest,
records all relevant SkillData/WeaponData references, literal helper/record key
sites and the reviewed dynamic SkillData write sites. The full source test first
verifies the entire upstream source inventory and rejects startup overrides or
unmanifested Lua files. This is a reviewed proof for these exact bytes, not a
claim to decide the semantics of arbitrary future Lua. Any changed source domain
requires renewed review and acquisition. Semantic negative controls independently
reject changed source metadata, callbacks and inheritance suppliers even when
the acquisition recomputes the modified file hashes.

## Why the selected source is intrinsic

1. `CalcActiveSkill.lua:903–1022` creates the minion from the summon definition.
   The constructed summon has exactly `RaisedSkeletonSniper` in minionList and
   no minionUses or minionHasItemSet. Saved minion/item-set choices therefore
   cannot select a different source for this declaration. The only constructed
   minion item-use declaration is independently retained in the inventory.
2. `CalcActiveSkill.lua:986–989` contains two preceding replacement branches.
   Iron Mass explicitly requires `RaisedSkeleton`, a different profile. The
   bow/quiver branch reads minionUseBowAndQuiver, for which there is no supplier
   in the reviewed pinned producer domains below. It is an orphan upstream
   branch for the admitted input domain, not a native gameplay Boolean.
3. `CalcActiveSkill.lua:991–998` constructs the intrinsic weapon record from the
   selected minion profile. Its PhysicalMin/Max, AttackRate and CritChance are
   the four routed source quantities; their numerical producer proof is in the
   existing intrinsic attack packet, not this selection proof.
4. `CalcActiveSkill.lua:955,1001–1022` initializes an empty minion item list and
   restricts weapon replacement to the absent minionUses/itemSet declarations.
   `CalcPerform.lua:1122–1139` only fills that item list under those declarations.
   Consequently `CalcOffence.lua:774–789` cannot apply Energy Blade's dynamic
   actor weapon mutation: it requires an item in that slot.
5. Basic has one stat set with attack/projectile flags, no unarmed flag, no parts,
   and no source execution callback. `CalcActiveSkill.lua:461–478,559–564` also
   reads parts and mapped skillFlag fields; both are included in the raw graph
   inventory. Supports' addFlags are suppressed for child skills with summonSkill
   (`CalcActiveSkill.lua:224`); the complete declaration inventory additionally
   rejects a new unarmed source flag. `CalcOffence.lua:701–705` dispatches only
   the selected grantedEffect callback, not arbitrary other catalog callbacks.
   Thus the main-hand copy at `CalcOffence.lua:2526` retains the intrinsic source;
   the unarmed replacement at 2528 does not apply.

## Supplier-domain closure for the orphan inheritance key

`CalcActiveSkill.lua:896–900`, `CalcSetup.lua:2218–2224` and
`CalcOffence.lua:738–744` transport existing SkillData LIST keys. They do not
invent key names. The complete constructed inventory covers both direct and
nested records, including ExtraSkillMod and MinionModifier payloads. The
`ExtraSkillMod` insertion at CalcActiveSkill 840–842 likewise copies a payload.
Parent/global/minion modifier transport preserves the key; numerical scaling
does not rename it.

`Data.lua:69–70` is the single generic runtime makeSkillDataMod constructor.
Every catalog `skill(...)` call has a literal first argument; computed first
arguments require review and are rejected. Direct LIST constructors in the
parser and configuration callbacks also declare literal keys. The global and
per-stat-set maps are both visited, including the Data.lua 913–924 metatable that
copies global mappings and annotates provenance/tags. ExtraSkillStat lookup
through those maps (`CalcActiveSkill.lua:80–140`) introduces no new key domain.

The remaining indexed SkillData writes are explicitly inventoried: resource
reservation keys in CalcDefence, existing LIST-key transport in CalcActiveSkill,
CalcSetup and CalcOffence, explosion percentage keys in Skills/other, and
damage-type endpoint/bonus keys in CalcOffence. They cannot construct the orphan
inheritance name. The only source references to that name are the two reads at
CalcActiveSkill 986 and CalcPerform 1107.

Parser recipes and ModCache, ConfigOptions callbacks, all loaded data definitions,
modifier transport and runtime modules are pinned together. ConfigTab 1177–1197
invokes registered configuration callbacks; custom text 1207–1235 goes through
parseMod and cannot supply arbitrary modifier tables. SkillsTab 303–395 reads
explicit saved attributes and resolves existing skill/gem definitions. A saved
skill ID, minion selection or item-set number cannot inject a raw grantedEffect,
callback or SkillData table. This proof does not admit arbitrary injected Lua or
replacement source data under the old pinned release identity.

## Bonus endpoints and scope

Corpse/explosion code at CalcOffence 2413–2414 writes skillData bonus endpoints.
The attack passes at 2526/2556 copy weapon records; only the nonattack pass at
2593 selects skillData. MinionInstability's callback similarly writes its own
skillData FireBonus fields. Those do not become Sniper weapon bonus endpoints.
Item.lua 2502–2503 has an additional dynamic WeaponData writer, whose reviewed
parser/cache keys are ordinary endpoints, critical chance and weapon flags,
not bonus endpoints. None of this publishes a universal Action zero or proves
the unfinished final base coefficient and flat-added modifier inputs.

The retained preconversion source03 report is corroborating evidence of the
selected original pass, independently replayed in both JIT modes. Its hash is
pinned here. Observed output alone is not the supplier-domain proof above.
