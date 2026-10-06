# Pre-Amulet bonus aggregation

**Status:** Source02, publication01 and all five native integration checks pass.

This packet continues `runs/owned-offering-final-inputs-03/package`, input
`9a2d363f69c9895f3b6dc87165a3dda74f859a753d936ee2b2cca057608c01d4`.
It supplies the missing reducer for existing Player Stat `32e4`: sum its
pre-Amulet Add contributions in percent units and derive the corresponding
scalar value. The explicit Player receiver instantiates that stat-owned program
independently of class, skill, item count or selected metric. Existing Mystic
Attunement Passive `1b09` supplies its already-published 25-percent contribution.

This is an ordinary owned rule and receiver, using existing operations V20 and
stages V4. No source-language types, special engine opcode, definition, slot,
table or alternative evaluator is introduced. The reducer's own program
inventory is Complete; that does not complete its incoming contributor set.
All existing item, Passive and Gem owner closures remain unchanged. Unknown
incoming owners or memberships still prevent execution of a complete build.

The source reads the percentage before copying the active Amulet modifiers.
The authored stage fragment therefore freezes the contribution stream before
the reducer, and freezes the scalar before the copy program. Stage freezing
rejects a late writer; it does not silently filter out late contributions.
Future sources must explicitly distinguish pre-copy and post-copy values.
No copied percentage may feed back into this snapshot. Empty Sum is typed zero
only for a proved complete empty input set; missing coverage is not zero.

`snapshot-authoring.json` describes explicit offline endpoint authoring. The
append-only public migration API intentionally cannot add a Complete owner to
an existing subject or grow its Complete receiver inventory. Authoring uses an
unpublished Partial-owner stage for checked release rebinding, then full endpoint
assembly to add only the reviewed Complete stat owner and exact receiver. Whole
input inverses must prove every other program, receiver, closure, schema,
allocation and import policy is preserved. No public migration permission is
relaxed. Readiness and source evidence are authoring references, not a separately
loadable evaluation bundle.

The source witness must authenticate the actual original Sum/store chain at
the consumer, including its complete current same-name buckets and physical
copy transport. Five originals, independent repeats, actual Passive controls,
item controls and uninstrumented comparisons retain the fixed lifecycle and
both JIT modes. Custom item lines test arithmetic and ordering; they do not
authorize item rolls or establish a complete candidate domain.

Native integration replaces the earlier literal snapshot boundary with the
published reducer and actual Passive program on the same graph as item rules,
source preparation, final Offering inputs and table lookup. Retain independent
source identity, missing/Partial contributor refusal, late-writer rejection,
scratch reuse and parallel replay. The old bounded fixture remains a numerical
reference for the preceding checkpoint; it is not a production fallback.

Source02 passes nineteen cases, three fixed stages and both JIT modes in
452.78 seconds. All four raw/compared reports are byte-identical at 64,324,391
bytes, SHA256 447d75d7014819fb20b730329e15fa918953b99b306ab9a482043e1289b7c7db.
The committed eight-case selected-environment projection is 1,311,277 bytes,
SHA256 37422bea16928ee93cfabf9dd51c3e6c0c4683ff5eaf68e9057c7f0bf5e78b2e.
The separate historical Amulet witness remains unchanged; it retains useful
four-lane/source-line evidence while this witness extends actual Passive and
five-original breadth.

Source01 exposed an invalid control: appending Ritualist notable 7068 to a
different class without a connected ascendancy path caused ordinary PoB loading
to prune it. Source02 uses connected Huntress/Ritualist controls and verifies
actual class, ancestry and original node-builder transport. Removing the notable
restores that connected diagnostic baseline, not Original05. Both copies of each
original remain byte-identical; no calculation function, lifecycle or numerical
tolerance changed. Source01 remains an immutable diagnostic receipt.

Publication01 passes in 28.68 seconds and preserves all five original sources,
local identities and 110 query rows. The release rebuild is byte-identical.
Five native tests pass in 10.77 seconds, including the actual 25% contribution, both support tiers,
individual item-copy rounding/removal, independent raw Offering inputs, exact
missing/Partial refusals, stage/feedback rejection, scratch reuse and Rayon.
The two ordinary authoring tests, seven historical Offering tests, five Bidding
tests, strict workspace Clippy and eight package format checks pass.

## Reproduction

Source evidence is optional PoB execution. With pinned submodule files present,
set POE_AMULET_BONUS_SNAPSHOT_SOURCE_OUT to a fresh directory and run:

    cargo test -p poe-optimizer-pob --test owned_djinn_support_preparation_source amulet_bonus_snapshot::amulet_snapshot_observes_original_sum_and_copy_boundary --locked -- --exact --ignored --nocapture

Publication authenticates the certificate's exact raw reports in
runs/owned-amulet-bonus-snapshot-source-02. A replacement report requires a
reviewed certificate/authoring digest; do not overwrite an earlier receipt.
Set POE_OPTIMIZER_TEST_AMULET_SNAPSHOT_PRIOR to the checked Offering predecessor
and POE_OPTIMIZER_TEST_AMULET_SNAPSHOT_OUTPUT to a new directory, then run:

    cargo test -p poe-optimizer-cli --test owned_amulet_bonus_snapshot publish_amulet_bonus_snapshot_preserving_five_originals --locked -- --exact --ignored --nocapture

For native replay, set POE_OPTIMIZER_TEST_AMULET_SNAPSHOT_RELEASE to the
published package and run:

    cargo test -p poe-optimizer-cli --test owned_amulet_bonus_snapshot native::amulet_snapshot_ --locked -- --ignored --nocapture

Use the optimized test profile with debug assertions/overflow checks retained.
Ordinary tests need only committed artifacts and perform no PoB execution.
Serialize Cargo and freeze source/data writers. The checked baseline is
runs/owned-amulet-bonus-snapshot-01/package, input
4526f13e139cc0bc24f99ba12382331afbda4513677118fa377f3b31c8905cbe,
with 18 files / 60,842,780 bytes / 103 provenance rows and no evaluation bundle.

The subsequent late-slot Source02 witness proves that its diagnostic copies
do not change already-prepared gem inputs. Focus's separate merge branch still
needs its own dynamic proof. The next substantive `30ca` gap is ordinary
recipient applicability: Necromantic Talisman diverts original Amulet records,
while the current direct native rule delivers to Player unconditionally.
See the [current plan](../../../../../docs/implementation.md) for those separate
obligations and the unchanged contributor/owner gates. Complete native original
builds remain **0/5**.
