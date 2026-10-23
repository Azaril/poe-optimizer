# Initial skill-type facts

This evidence supports the maintained Skeletal Sniper producer in
[`skeletal-sniper.rule`](../migration-source/rules/initial-skill-types/skeletal-sniper.rule).
It projects the initial source `skillTypes` and optional `minionSkillTypes` into
the 34 concrete symbols declared by the reviewed
[`preparation.json`](../djinn-support-preparation/preparation.json). The
[`bindings.json`](../djinn-support-preparation/bindings.json) correspondence supplies
each source type name and ID. This vocabulary is local to that preparation
artifact; it does not describe every PoB skill type.

[`evidence.json`](evidence.json) records the source pin
`3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`, the canonical source-manifest digest,
and seven source-file hashes checked against that manifest. Source-file hashes
use UTF-8 with CRLF normalized to LF, matching the existing PoB source contract.
The evidence retains raw names in source order, exact declaration lines,
projected keys in vocabulary order, excluded names, and field presence. Its
vocabulary rows preserve the source IDs and spellings, including
`MinionsAreUndamagable`. DSL keys replace the reviewed symbol suffix's hyphens
with underscores; they do not infer new source-name correspondence.

Sniper's own set contains twelve raw types. Seven enter the reviewed vocabulary:
`CommandableMinion`, `CreatesMinion`, `HasReservation`, `Minion`,
`MinionsCanExplode`, `Persistent`, and `Physical`. The excluded raw names are
`CreatesUndeadMinion`, `CreatesSkeletonMinion`, `MultipleReservation`, `Chaos`,
and `Fire`. Its minion field is present and all five raw members enter the
vocabulary: `Attack`, `Projectile`, `Duration`, `Area`, and `CreatesGroundEffect`.
An excluded name is outside this projection, not evidence that the source lacks
that type.

The controls preserve different initial sets and optional-field states:

| Source skill | Own projected / raw | Minion field | Minion projected / raw |
| --- | --- | --- | --- |
| `SummonSkeletalSnipersPlayer` | 7 / 12 | present | 5 / 5 |
| `TwisterPlayer` | 5 / 12 | absent | — |
| `SummonSkeletalClericsPlayer` | 5 / 8 | present | 2 / 2 |
| `HealSkeletonClericMinion` | 4 / 4 | absent | — |
| `SummonSandDjinnPlayer` | 8 / 8 | present | 8 / 8 |
| `SummonWaterDjinnPlayer` | 8 / 8 | present | 8 / 8 |

Twister's `CannotTerrainChain` is excluded; it is not the vocabulary's
`CannotChain`. Cleric's minion set contains `Cooldown` and `Spell`, while Heal's
own set contains `Buff`, `Duration`, `Spell`, and `AttackInPlace`. The child's
initial facts must remain distinct from its parent's prepared support context.
Only Sniper receives a maintained producer in this checkpoint. The other rows
are evidence and component-test controls, not production coverage declarations.

The [declarations](../migration-source/declarations/initial-skill-types.kdl)
allocate 69 shared Boolean Stats targeting Skill: 34
`stat.initial_skill_type.<key>` memberships, 34
`stat.initial_minion_type.<key>` memberships, and
`stat.initial_minion_types_present`. The [names input](../migration-source/names/initial-skill-types.kdl)
contains only the namespace; the existing declaration allocator supplies the
new addresses. Sniper's program has an explicit derive for every Stat, using
two shared Boolean literals. The values describe initial source facts before
support preparation; they contain no observed admission or final type results.

The presence channel preserves three different outcomes in the existing
computed-input adapter. Known false means the minion field is absent and member
channels are not demanded. Known true with every member false means a present
empty set. A missing presence producer remains unavailable. The evidence's empty
raw arrays for absent fields are paired with `present: false` and must not be
read as present empty sets.

These scalar Stats use the current DSL and checked Engine. They do not add a
set-valued computed type or replace support-input bindings. Producer readiness,
effective support values, prepared types, receiver and summoner context, and a
complete release evaluation bundle remain separate requirements. This initial
projection establishes no whole-build completion claim.
