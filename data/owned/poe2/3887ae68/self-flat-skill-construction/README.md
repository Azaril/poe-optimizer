# Selected skill construction and observed buff transport

This offline proof closes a finite source-review boundary for Original 05: the
selected skill/support recipes contain no direct or nested `PhysicalMin`,
`PhysicalMax`, or `ExtraSkillStat` records, and the captured construction calls
preserve the complete local records while accounting for records moved into
global-effect buff lists. It does not complete a native contribution query or
make the full build evaluable.

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

`ExtraSkillStat` suppliers and arbitrary Party inputs, inherited
Player/item/passive/configuration stores, dynamic child and parent-weapon
factories, additional global recipients and activation branches remain separate
obligations. In particular, a complete captured local record is not proof that
an inherited store or an unexecuted factory cannot later supply flat damage.
Current native owner inventories, supplier completeness, Action contribution
queries and all build/request coverage are unchanged.

The recipe join authenticates the exact selected table bodies and their consumed
definition identities. It does not reevaluate Lua recipes in Rust or claim
recipe-value calculation parity. The supplied complete acquisition is sealed
after the semantic checks; a coherent edited before/after value can satisfy the
transport law but still fails this evidence seal.

The next useful boundary is the complete inherited-store input to the actual
`MinionModifier` receiving calls, joined to exact source occurrences and recipient
filters (`CalcPerform` calls at 1854, 3123, 3377 and 3384). That requires
source/delivery authority beyond the finite local construction proof here. No
positive native producer or empty-query completion is justified by this packet
alone.

## Validation

The focused Rust target is `owned_self_flat_skill_construction`. Its ordinary
tests are portable and use only committed evidence. The ignored retention test
requires `POE_SELF_FLAT_SKILL_RETAIN=1`; it runs no PoB VM and writes only this
packet's reviewed projection. A test-only mutex bounds simultaneous large-report
clones during the mutation checks.

Final validation:

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
