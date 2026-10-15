# Selected skill construction and observed buff transport

This offline proof closes a finite source-review boundary for Original 05: the
selected skill/support recipes contain no direct or nested `PhysicalMin`,
`PhysicalMax`, or `ExtraSkillStat` records, and the captured construction calls
preserve the complete local records while accounting for records moved into
global-effect buff lists. It does not complete a native contribution query or
make the full build evaluable.

## Sniper child ExtraSkillStat lifecycle

The separate `child-review.json` extends this packet with a source-level
deduction for the 18 unchanged Sniper Basic Attack constructor pairs. It keeps
the original `authority.json`, `bodies.json` and `review.json` byte-for-byte,
because other proofs authenticate those artifacts. No new PoB load or query
observation was performed.

Both reviewed construction paths, `calcs.perform` and `calcs.copyActiveSkill`,
replace the minion's modifier database with a fresh `ModDB()` before calling
`createMinionSkills`. The complete database/store constructors establish an
empty local database with no store parent. The child's new modifier list
inherits only that database. The Actor's parent relationship to the Player
does not establish a modifier-store parent.

The exact retained Basic Attack calls have one active effect, no supports, no
source instance or socket group, quality zero, and level one. The complete
construction prefix through the active merge at line 795 writes only reviewed
fixed names on this path. The source-gem property guard cannot enter without a
source instance. The selected recipe and its resolved mappings contain no
`ExtraSkillStat`, `PhysicalMin` or `PhysicalMax` writer. The original List
implementations therefore have no matching local or parent record to return
at that direct child merge. This deduction uses the lifecycle and full source
bodies; it does not interpret a later empty observation as an earlier absence.

`child-bodies.json` authenticates 17 spans in seven complete files, including
the constructors, both callers, the entire pre-merge prefix, stat preparation,
List traversal and the later inherited-delivery boundary. The complete
`CalcTools.lua` archive is normalized to the exact LF bytes authenticated by the
existing upstream manifest. The other six archives are reused. The existing
upstream license is retained at
`../configuration-accounting/evidence/pob/LICENSE.md`.

The original global stat-map fallback remains capable of resolving additional
keys. This proof excludes their input at the reviewed child merge; it does not
claim that arbitrary additional keys are harmless. Later minion-store writes,
parent-weapon factories, Party inputs, other skill/support selections and
source-to-native contributor membership remain separate obligations. No native
query membership or final build coverage is promoted.

The new ordinary tests reject coherently rehashed parent-store inheritance,
changed caller bodies, substituted child identities, source instances or
support effects, changed recipes, and coherent but unauthenticated observations.
The optional `retain_sniper_child_extra_stat_lifecycle` test only reconstructs
the new offline report from existing evidence.

The proof reuses the authenticated selected-domain recipes, exact typed source
bindings, Mana supplier source-02 acquisitions, and self-flat runtime-gate
observations. It stores small authority/body/review documents rather than copying
the large raw reports. All source files remain pinned to PoB revision `3887ae68`
through the original source manifest; their existing archives and upstream
license are retained in the referenced packets.

## Checked boundary

- The complete 40-recipe inventory covers 28 active/generated/child effects and
  12 support definitions. All base, stat-set, quality and resolved mapping bodies
  are inspected recursively, including zero values, inactive tags, nested
  `MinionModifier` payloads and unselected stat-set data. Callbacks, unresolved
  aliases and external-stat records require independent review.
- Each retained observed trace has 768 complete before/after constructor pairs,
  including 18 Sniper Basic Attack pairs. Exact authored/generated source joins,
  supplied Actor profiles and child effect membership come from the existing
  typed binding proof; positions in an anonymous traversal are not identities.
- Every ordinary local record preserves order, value and multiplicity across
  its exact call/return pair. A counted full-body comparison accounts for every
  removed `GlobalEffect` record in the resulting buff lists. That comparison
  proves conservation, not arithmetic associativity or execution order.
- The separate runtime-gate captures contain 36 Player buff visits, 18 supplied
  child buff visits, no child buff merges, and 28 group deliveries. Visits join
  on complete source, Actor and payload. Group deliveries establish payload
  correspondence and the observed receiving store; same-name record order and
  duplicates are preserved. They do **not** identify a unique delivering source
  when several source occurrences have the same buff name and payload.
- Each per-call selected minion receiving store joins to its exact typed
  summoner, Actor profile and observed child Skill constructor. The captures
  include ten minion deliveries across six supplied Actors (two for Sniper),
  plus eighteen Enemy deliveries. Unavailable traversal flags are never
  interpreted as false. No new recipient authority is inferred.

The source review retains 19 exact body spans from five authenticated complete
files. They cover scalar/level value merging, stat mapping and base records,
support selection, the observed active/support merge callers, global-effect
routing, child construction, buff grouping and the generic list/store copy
operations. These operations preserve modifier names within the reviewed paths;
that does not supply missing inputs to those paths or prove every caller branch.

Both constructor traces were observed with JIT disabled. Their existing source
acquisition packet also retains independent unhooked JIT-off/JIT-on scalar
controls. This proof does not relabel either observed trace as JIT-enabled.
The retained second constructor trace enumerates Pain Offering's Damage/Speed
records in the opposite cross-stat order from the runtime-gate trace. At the
already-reviewed Player buff inventory and group-delivery paths, the existing
source comparison policy uses exact-name buckets, retaining full bodies,
multiplicity and within-name order. Every other visit field remains exact; the
child-buff visit list does not receive this relaxation.

## Explicitly unfinished

The bounded child lifecycle above excludes `ExtraSkillStat` only at the direct
Sniper construction merge. Arbitrary Party inputs, other child/support selections,
later direct minion-store writers, dynamic parent-weapon factories, additional
global recipients and activation branches remain separate obligations. A complete
captured local record does not prove that an unexecuted factory or later store
write cannot supply flat damage.
Current native owner inventories, supplier completeness, Action contribution
queries and all build/request coverage are unchanged.

The recipe join authenticates the exact selected table bodies and their consumed
definition identities. It does not reevaluate Lua recipes in Rust or claim
recipe-value calculation parity. The supplied complete acquisition is sealed
after the semantic checks; a coherent edited before/after value can satisfy the
transport law but still fails this evidence seal.

The separate [inherited delivery proof](../self-flat-inherited-minion-delivery/README.md)
now checks the actual `MinionModifier` receiving calls and skipped gates at
`CalcPerform` 1854, 3123, 3377 and 3384, with exact source/recipient and original
List/AddMod correspondence. That proof does not complete every minion-store
writer. The remaining native work must join the reviewed direct, inherited and
dynamic source domains to exact contributor membership; neither packet alone
authorizes an empty native query or a complete build.

## Validation

The focused Rust target is `owned_self_flat_skill_construction`. Its ordinary
tests are portable and use only committed evidence. The ignored retention test
requires `POE_SELF_FLAT_SKILL_RETAIN=1`; it runs no PoB VM and writes only this
packet's reviewed projection. A test-only mutex bounds simultaneous large-report
clones during the mutation checks.

Child lifecycle validation:

- Offline retention: one passed in 9.19 seconds,
  `runs/owned-self-flat-child-lifecycle-retain-01.log`.
- Current ordinary target: fourteen passed, two intentional retention-only
  ignores, in 21.47 seconds, `runs/owned-self-flat-child-lifecycle-tests-01.log`.
- Strict target Clippy: passed in 0.82 seconds,
  `runs/owned-self-flat-child-lifecycle-clippy-01.log`.

The child report retains 16 MAIN, one CALCS and one CALCULATOR construction pair
in each of the two independent observed traces. Use
`POE_SELF_FLAT_CHILD_RETAIN=1` for its explicit offline retention test.

Original constructor proof validation (unchanged evidence):

- Offline retention: one passed in 9.27 seconds,
  `runs/owned-self-flat-skill-retain-03.log`.
- Ordinary tests: ten passed, one intentional retention-only ignore, in 17.10
  seconds, `runs/owned-self-flat-skill-ordinary-02.log`.
- Strict target Clippy: passed in 0.62 seconds,
  `runs/owned-self-flat-skill-clippy-02.log`.

The 69,320-byte `review.json` has SHA-256
`b2c444e057d10a0a18308b3cbed3a2a52abd5f811f087800b6dcb56591b04533`.
Each observed trace accounts for 6,771 ordinary records and 123 global records
moved into 60 buff lists, across 768 constructor pairs and 120 exact context
frames. Mutation controls refuse malformed positions, missing source bodies,
changed source/Actor joins, lost or duplicated records, changed within-stat
ordering, unreviewed nested records/callbacks, coherent edited constructor values
and swapped valid recipients with identical payloads.

The initial two retention failures are retained: the first incorrectly assumed
all receiving minions were Snipers; the second exposed the already-reviewed
Pain Offering cross-stat inventory permutation. The finished checks retain every
actual recipient and apply only the existing path-specific comparison policy.
No raw source observation was changed or reacquired.
