# Selected Sniper later-helper source proof

This is a finite offline source proof for unchanged Original05's supplied Sniper
Basic Attack. It introduces no native rules, data IDs, coverage closure, numerical
parity claim, or new PoB execution.

The single authenticated dependency chain is the existing minion-store proof,
inherited receiving proof, local/child construction proof, radius proof, and
supplier acquisition. The old artifacts and their identities remain unchanged.
`bodies.json` seals eleven complete source files and twenty-one reviewed spans;
only `CalcTriggers.lua.gz` is newly archived here. The source manifest explicitly
defines CRLF-to-LF source hashing. The archives are upstream Path of Building
code; see the retained [copyright and license](../configuration-accounting/evidence/pob/LICENSE.md).

The review distinguishes modifier stores from Actor output, condition and
multiplier tables. Attributes, charges and misc have fixed modifier names.
Defence's computed names use closed resistance/resource domains. Defence
estimation, breakdown and numeric callees modify outputs or temporary tables,
not arbitrary modifier payload names. The writer-site list is a regression
checksum of the manually reviewed complete source, not a Lua parser or a general
name-based exclusion mechanism.

`EnemyModifier` is an arbitrary nested-payload transport to `actor.enemy.modDB`.
The original Player/Enemy relation and exact friendly minion constructor show
that these helper invocations target Enemy or Player, not this Sniper. This does
not prove arbitrary parent-store readers harmless. Player offence has two
explicit minion-store writers: `CritChance` and `CritMultiplier`; neither is
silently omitted from the proof.

The 18 Basic Attack construction pairs per retained JIT mode preserve the exact
source and recipient identities, no support or source-instance input, one effect,
empty socket group and skillData. Combined with the fresh constructor and full
pre-routing prefix, these derive no `triggeredBy` supplier; omission of a table
from scalar metadata is not treated as observed nil. The complete trigger
configuration table has no Basic Attack entry, and no alternative trigger-name
source remains. Consequently the pinned dispatch has no handler for this child.
This is a source deduction, not an observed dispatch event.

Reservation can append `ExtraAura` containing an arbitrary nested modifier. This
writer remains explicit. It runs after the same pass's ExtraAura delivery, and
the subsequently called trigger/offence modules do not consume that wrapper.
This proves only the current-pass ordering boundary. It does not prove absence,
arbitrary reused-environment behavior, or exclusion on a later pass. The actual
later Party/AffectedByAuraMod observation is a separate packet. Flask/Keystone,
Action-local factories and final native supplier admission also remain separate.

The ordinary target is `owned_self_flat_later_helpers`. The ignored
`retain_later_helper_review` test authors `review.json` only when
`POE_SELF_FLAT_LATER_HELPERS_RETAIN=1`; it reads retained evidence and never runs
PoB.

Validation: offline retention passed in 12.13 seconds; all seven ordinary tests
passed in 15.33 seconds; strict target Clippy passed in 1.32 seconds. Logs are
`runs/owned-self-flat-later-helpers-retain-01.log`,
`runs/owned-self-flat-later-helpers-ordinary-01.log` and
`runs/owned-self-flat-later-helpers-clippy-01.log`. Mutation controls include
coherent source/checksum changes, computed-name domains, Actor destinations,
trigger sources, erased generic edges, coverage overclaims, and coherent changes
outside the selected child that only the complete acquisition seal can reject.
