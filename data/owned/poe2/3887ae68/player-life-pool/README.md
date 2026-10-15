# Guarded Player Life consumer

This packet supplies the actual final Player Life formula on existing Stat
`311a`, using the existing Life unit `3119` and the three published contribution
queries. It adds six typed Stats (`3383`–`3388`): outgoing Life-to-Energy-Shield,
Armour and Evasion percentages; additional Life before scaling; additional Life
after scaling; and Chaos Inoculation. Seven checked queries preserve their
unreviewed supplier domains as Partial. One final Player receiver reads those
typed contribution queries directly using the existing native rule language. No runtime source name,
Lua type, importer or new engine operation is introduced.

The final receiver adds the four existing BASE groups, the two INC groups, and
reads the existing MORE group. It caps the sum of outgoing conversion at 100%,
without a lower clamp, then applies conversion, extra Life, increased/more
scaling and the post-scaling amount in the source formula's arithmetic order.
The actual ordinary rounding operand must be finite and at most `2^52`. Existing
native rounding followed by a minimum of 1 matches the reviewed source on that
domain; this is not a claim of standalone rounding equivalence everywhere.

Final-value precedence is Chaos Inoculation → present coherent override
(including 0) → ordinary arithmetic. Native Select is lazy. Original PoB executes
the ordinary resource loop before its later Chaos Inoculation assignment; this
packet preserves the final Life result, not that internal execution history.
Whole-plan coverage gates remain mandatory when a scalar branch is skipped.
The source's FullLife condition side effect and lowest-Life/Mana result are
separate responsibilities and are not implemented by this Stat receiver.

The final owner's program inventory remains Partial for the unproved
cross-group and conversion arithmetic-order domain. The equipment group's
bounded exact sum does not establish arbitrary regrouping with inherent,
intrinsic or reward sources. The new adjustment/override/flag queries remain
Partial-empty; their identities are not evidence of absent game suppliers.
All existing donor, recipient, global and input-inventory obligations remain.

## Evidence and validation boundary

The exact retained Player Life rounding witness provides 11 cases, two observed
JIT-disabled acquisitions and two uninstrumented references in each JIT mode:
66 fresh loads from the historical checkpoint. The archived reports are
compressed unchanged. Ordinary Rust tests authenticate every archive and
reconstruct each compact vector from 72 original calls per observed acquisition,
checking exact repeats and complete retained scalar/store projections against
both reference modes. No new source process is executed for this packet.

The first six cases are the unchanged Original05, level 91, and four robe-roll
controls. Five custom-modifier cases cover increased Life, more Life, Chaos
Inoculation, zero base and negative base. Those controls prove source behavior,
not obtainability. The witness supplies actual aggregate inputs and the original
round argument; the native consumer does not read measured operands at runtime.
Positive transfers and overrides are not source-backed by this witness and any
such native controls must be labeled synthetic. The original unhooked outputs
do not prove complete supplier coverage.

The all-five publication test is
`owned_player_life_pool::publish_player_life_pool_preserving_all_five_originals`.
It uses `POE_OPTIMIZER_TEST_PLAYER_LIFE_POOL_PRIOR` and a fresh
`POE_OPTIMIZER_TEST_PLAYER_LIFE_POOL_OUTPUT`, preserves all 110 saved query rows,
and authenticates the exact schema-only rebindings followed by the authored
owner, receiver and seven queries. Publication 02 passes in 35.56 seconds with
byte-identical rebuilding: `runs/owned-player-life-pool-publication-02/package`,
input `aedc17306523ec3c513f6e04c2d1937774705585b9c33c9c1cc9006b0c6452a1`.
All five selected issue counts remain 106/117/109/123/4, and no physical inventory
is completed. All 101 joined native tests pass against this published endpoint
in 65.06 seconds (`runs/owned-player-life-pool-joined-05.log`). The eight new
checks cover real sources deriving 1320 Life, input changes, all eleven retained
source controls, explicit synthetic adjustments/overrides, numeric limits,
unknown/Partial inputs, frozen stages, scratch/Rayon reuse and the saved
`reference-00` Life metric's known/unavailable results. The fixture explicitly
closes its finite domains; the published component retains every stated gap.
This packet does not claim a complete build.

The two ordinary source/authoring tests pass in 0.33 seconds after the final
shared-module cleanup (`runs/owned-player-life-pool-authoring-02.log`). Strict
Clippy for both affected integration targets passes in 4.48 seconds
(`runs/owned-player-life-pool-clippy-02.log`). The retained failed draft runs
diagnose metadata/fixture setup and the program-budget limit; they introduce no
source numerical exception or tolerance.

The first publication attempt exceeded the existing compiler's 8,192-program
limit: its predecessor contains 8,188 owner programs plus one effect application,
which consumes the same budget. Six redundant
pass-through receivers were removed in favor of direct checked queries; the
packet now adds only one owner program, using 8,190 budget entries and leaving
two available. The compiler budget is unchanged and the
failed attempt remains diagnostic evidence.

Original source evidence derives from Path of Building Community PoE2 revision
`3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`. The unchanged upstream license is
retained in `../mana-supplier-sources/upstream-LICENSE.md.gz`.
