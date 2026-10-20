# Reviewed support structural inventories

This packet refines 11 existing Gem definitions against input
`091556fee21e868c21ac3c2a61390c3e90c5900ab4576552c90a46a9fcc63ffd`.
It closes 33 empty direct `grants`, `actors`, and `skill_grants`
inventories and 11 empty potential `skills` inventories. It adds no IDs or
members. The exact original descriptors and import identity mappings are
retained in `dependencies.json`; `migration.json` uses the current v5 migration,
schema 7, and operations v28 contract.

The reviewed families are Bidding II, Encroaching Ground, Overabundance I,
Last Gasp, Magnified Area I and II, Muster, Prolonged Duration I,
Rapid Casting I, Short Fuse I, and Spell Echo. The finite evidence checker
joins each owned Gem to its exact external game/variant identity, winning
source declarations, complete constructed effect list, and complete retained
recipe. The full recipe contributes 18 mapping rows, including four null
mappings. The declaration names and source identities in `evidence.json`
are authoritative if display names change.

All 15 source archives retain complete LF-normalized source files authenticated
against the pinned PoB revision and source manifest. Ordinary Rust tests do not
load Lua, use the vendor checkout, or require ignored `runs` artifacts. The
checked-in identity catalog and retained source-recipe artifact are separately
authenticated. Source spans are indexes into authenticated whole files, not
substitutes for them.

The factory review covers the complete `setupGem` constructor through
`Data.lua:1068`, its additional effects and Barbs auxiliary path, support
construction and executable skill filtering in `CalcSetup.lua`, and minion
construction in `CalcActiveSkill.lua`. In particular, `addMinionList` is checked
in the full source even though the older recipe metadata did not capture that
field. Exact primary effects are supports; their full effect lists contain no
active effects or unreviewed auxiliaries. The Gem inventories describe only
their enclosing owner, not the topology of supported recipient skills.

Null mappings do not establish absence of a mechanic. Encroaching Ground's
two area-growth values are bound to the actual `gem_stat_descriptions` record
about surfaces already created by supported skills; Last Gasp's duration is
bound to delayed death of existing minions; Magnified Area I's value is bound
to area damage on supported skills. The last classification does not depend
on its saved value being zero. The source description records and their scope
selection/export code are authenticated. Original game CSD files and an
independent game execution oracle are not retained here, and this packet does
not establish any numerical formula.

Other Gem declaration inventories, every descendant Skill/Actor/Action
inventory, numerical rules, source-domain coverage, support applicability,
and complete-build status remain unchanged. Frost Nexus, Frost Bomb, Ice Nova,
Pain Offering, and unresolved minion-command families are outside this packet.
In particular, PoB's absence of a Pain Offering minion profile cannot prove
that the explicitly described bone-spike minion does not exist.

The joint publisher consumes `checked_migration()` and `prior_descriptors()`,
checks the actual predecessor, and proves a complete schema inverse before
reimporting the five unchanged development builds. The ordinary target
`owned_support_structural_inventories` checks the authored delta and rejects
missing identities, extra effects, Barbs auxiliaries, unknown factories,
missing or duplicated mapping rows, callbacks, aliases, structural modifiers,
wrong description scopes, and unrelated closure changes. This is a bounded
offline proof review, not a new runtime parser or general absence classifier.
