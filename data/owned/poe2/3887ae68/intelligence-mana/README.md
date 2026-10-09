# Intelligence-derived Player Mana

The shared Player Actor `332a` now contributes inherent Mana through the existing
`29f9` channel and `0003` unit. Its injected program reads final Intelligence
`1d30`, contributes two Mana per point, and applies the existing inherent-bonus
doubling flag `3318`. Global suppression `3315` or either new Intelligence-specific
suppression flag disables emission. Disabled absence and enabled zero remain
different results. Missing active inputs never supply a neutral value.

Only two Boolean Stats are allocated: `3355` disables inherent Intelligence
bonuses and `3356` disables the Intelligence-to-Mana bonus. Each has an ordinary
checked `Any` query, reducer and Player receiver. The queries guard a complete
empty **currently admitted producer domain**, not universal game absence.
Publication censuses every existing ordinary/application contribution; a future
unlisted writer must fail membership even when inactive or false. Real owners,
global coverage and unconverted item/passive sources retain their existing gaps.
The parsed source controls do not authorize new game-source families.

The arithmetic and guard are ordinary rule nodes in one contribution program.
No extra intermediate Stat is needed: later resource consumers can query the
exact existing-Actor producer through the accepted contribution graph. There is
no new calculation engine, opcode, Lua behavior or development-format fallback.

## Independent evidence

The optional Rust source test runs unchanged pinned PoB with parsed custom-modifier
controls. It composes the existing intrinsic-Mana observer with a separate
Intelligence/flag observer. Both call original methods; neither replaces a
calculation or injects a computed result. The source emission is taken from actual
Mana records and an original source-filtered Sum.

Seventeen cases cover all five originals, ordinary/duplicate doubling, each of
three independent suppressions and its combination with doubling, enabled zero
Intelligence, doubled zero, a fresh replay and a warm suppressed-to-original
transition. A second independent warm replay also matches. Each JIT mode performs
20 complete loads; its 390,703-byte report has SHA-256
`df8fdf5d7c5082096023295f9170679ba9bdc3cc22d0c54d2ec853af462bc6e2`.
The JIT reports are byte-identical. Tests finish in 65.50s.

`player_resource_source.rs` shares complete-source loading, original-corpus
authentication and bounded process supervision with the intrinsic-Mana test.
That refactor reproduces the previous intrinsic report byte for byte; its
independent rerun passes in 41.84s. Older pinned observers remain unchanged.

The ordinary native test extends the existing finite joined Sniper replay with
the actual published programs, queries and receivers. Its baseline retains the
calculated attribute path: Intelligence 105 produces 210 inherent Mana alongside
398 intrinsic Mana at imported level 92. The other source cases explicitly supply
final Intelligence and synthetic flag contributors inside the test domain only.
They test the numerical consumer, not additional real-build completeness.

Checks distinguish disabled/zero/unknown, reject an unlisted false writer, keep
partial query coverage unavailable, and preserve fresh/reused/four-worker results.
An explicit test-authoring helper rebinds changed artifact identities through the
normal validating constructors. Normal replay compilation still rejects stale
identities and never repairs inputs implicitly.

## Publication and next dependency

The predecessor is `runs/owned-player-intrinsic-mana-publication-03/package`.
Publication02 passes all-five import preservation and byte-identical reconstruction
of 18 artifacts; 110 queries and selected input counts 107/117/109/123/4 remain.
The canonical successor is `runs/owned-intelligence-mana-publication-02/package`,
input `ef3f45bb8aa6b755f21d79aeac6740bb469cc6406e8cce8ce4f11ce5147b7c56`.
Operations remain V25; the next free definition is `3357`.

Run ordinary checks with `cargo test --locked -p poe-optimizer-cli --test
owned_intelligence_mana`. The ignored publication test uses
`POE_OPTIMIZER_TEST_INTELLIGENCE_MANA_PRIOR` and a fresh
`POE_OPTIMIZER_TEST_INTELLIGENCE_MANA_OUTPUT`. Source reacquisition uses the PoB
target of the same name and a fresh `POE_OPTIMIZER_TEST_INTELLIGENCE_MANA_SOURCE_OUT`.

Final Mana still needs checked resource aggregation and validated final arithmetic.
Original05's source also contains a 5% quest increase; its observed final pool is
638. Those observations guide validation, not runtime literals. No final Mana
metric, whole-build completion or optimizer-ready candidate domain is claimed.
