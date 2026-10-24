# Revival producers

These authored mechanics use the existing contribution graph. Three generic
channels admit Skill and Actor recipients: Boolean `stat.revival.enabled`,
seconds `stat.revival.base_override`, and percentage points
`stat.revival.speed_increased`. Local and inherited contributions use the same
channel identity; exact recipients keep them separate. Eligibility does not
create a writer or imply a default value.

The [packet](../migration-source/packets/revival-producers.kdl) contains seven
program instances from three DSL bodies. The exact Sniper, Skeletal Arsonist,
Skeletal Frost Mage and Skeletal Reaver summoning Skills each contribute a true
flag and a twelve-second override to themselves. Each query member retains its
own physical Gem's declared Skill grant. The Restless Dead allocation contributes
25 increased revival speed to Player. Two distinct small passive allocations
each contribute 5; sharing a source body does not merge occurrences. There is no
native Rust dispatch by a skill or item name.

The [queries](../migration-source/contribution-queries/revival-producers.kdl)
retain exact Skill-grant and allocation origins. Every incoming group and the
query inventory remain Partial. False and zero are fold identities, never
evidence of absent modifiers. The optional override has no empty value. Numeric
ordering remains an explicit unresolved domain. No final revival time, fallback,
MORE multiplier, readiness classification or complete-build metric is introduced.

The retained PoB source pin is `3887ae68`. Source proof reuses authenticated full
archives and existing exact import mappings, rather than acquiring a second copy:

- `self-flat-minion-store/act_int.lua.gz` contains all four complete summoning
  definitions. The existing skeletal source proof also authenticates their
  winning definitions, physical Gem supplies and exact import mappings.
- `support-structural-inventories/sources/src_Data_SkillStatMap.lua.gz` binds
  `RevivingMinion` and converts the 12,000-millisecond base value to seconds.
- The existing passive tree archives, raw tree catalog, parser and exact mapping
  bind nodes 38972, 28573 and 44255 to their complete source records and actual
  Player contributions. Display names alone are not source authority.

The maintained Rust release tests check these source families and their mutations,
the resulting typed programs and descriptors, repeated component evaluation,
exact recipients, known query membership and retained uncertainty. Changed revive
flags, base values and paired Graft records are refused; the complete source
bodies retain all forty level and reservation rows. Crossed sibling supplies and
duplicate query members are rejected by the existing rule validator.

The original revival checkpoint added three Stats, four program instances and
three queries with one new outer gap. The subsequent skeletal extension reuses
those Stats and the intrinsic body, adding three program instances and six exact
query members without allocating names or IDs. Historical restoration removes
these explicit revival additions; unrelated data remains unchanged.

Original05 does not select these three passive nodes. They are real non-neutral
counterfactual sources; the unchanged build does select all four intrinsic
Skill producers. Their owners and incoming query groups remain Partial, and the
five selected input obligations remain unresolved.
The paired and conditional revival passives, post-stacking Sacrifice application,
complete composed-source census and numerical precision review remain follow-up
work. Graft and Skill-owned reservation obligations also remain required before
Sniper can be considered complete. See the [implementation plan](../../../../../docs/implementation.md).
