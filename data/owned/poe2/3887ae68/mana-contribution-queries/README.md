# Checked Mana contribution queries

This packet accounts for all five current owned writers to Mana `29f9`. It
uses existing V25 queries and changes no Core/Data/Engine contract, producer
program, definition, receiver, source importer or full-build coverage.

| Query | Groups | Potential sources |
| --- | --- | --- |
| `mana-base-contributions` | intrinsic, inherent | shared Player level and Intelligence programs |
| `mana-increased-contributions` | rewards, passives | reward `003e`, passive nodes `109b` and `18d8` |
| `mana-more-contributions` | sources | currently empty, guarded against unlisted writers |

Actor sources retain the existing once-per-Player application. Reward and
allocation origins preserve exact occurrences. The census examines every
published potential writer, including unselected and inactive writers, and
authenticates its complete program body. New or changed ordinary/application
writers require review; an empty MORE domain is not inferred from the five builds.
Existing global and owner Partial declarations remain mandatory.

The intrinsic, inherent and reward groups admit at most one potential effect
per recipient: equal semantic positions are rejected before activation. Each
passive has a distinct explicit position and contributes only its existing
integral literal (-10 or -30). Every subset sum is exactly representable; this
is a bounded proof, not a general floating-point associativity assumption.
An unlisted zero/inactive producer cannot disappear behind the empty identity.

The ordinary native resource target uses these exact programs and queries in
the existing explicitly finite Sniper replay. Additional passive topology is
finite test data, not a claim about legal full-tree routes. Its diagnostic
channels expose query values, not final Mana. The unchanged calculated
Intelligence path supplies 210 Mana, intrinsic Mana is 398 at level 92, and the
selected quest supplies 5% increased Mana. Neither those values nor a build
identity appears in production calculation code.

Run `cargo test --locked -p poe-optimizer-cli --test owned_intelligence_mana`.
For publication, set `POE_OPTIMIZER_TEST_MANA_QUERIES_PRIOR` to the checked
Intelligence-Mana package and `POE_OPTIMIZER_TEST_MANA_QUERIES_OUTPUT` to a fresh
directory, then run `publish_mana_queries_preserving_all_five_originals` with
`--ignored --exact`. The publication authenticates the full writer census,
proves the inverse change, rebuilds all artifacts and reimports all five builds.
Publication `runs/owned-mana-queries-publication-02` passes with all nine selected
tests in 24.38s; all 18 artifacts rebuild identically and original selected input
counts remain 107/117/109/123/4. The final ordinary run passes eight tests in
4.35s, including fresh/reused/unknown/four-worker query execution. Both new
native/PoB targets pass strict all-feature Clippy. No definition IDs are consumed.

## Final resource calculation evidence and remaining work

The optional `owned_mana_pool` PoB test runs the full original build and calls
the original `doActorLifeManaSpirit` twice while retaining its actual ModStore
inputs and output. It does not install copied calculations, wrappers or hooks.
Five originals, five parsed controls, a fresh repeat and warm restoration pass
with JIT off/on: 12 cases, 15 complete loads per mode. The two 98,915-byte reports
in `runs/owned-mana-pool-source-02` have SHA-256
`e73fa203a7f41c6be2faf9be4dc1a00094859039f263c9f47d3b644ffffb769b`.
The complete source test passes in 48.80s.

Original05's observed resource inputs are base 608, increased 5%, MORE 1,
zero conversion/extra/total and no override, yielding 638. The +2 flat control
produces 641 at the half boundary; 50% MORE produces 958; a negative result
clamps to one; a zero override remains zero. Other unchanged builds yield
876/630/881/858. These are source observations, not native full-pool parity.

The final consumer must explicitly account for conversion, extra/total Mana,
override presence/value and rounding. A neutral observation in five builds is
not complete contributor coverage. Source `round` uses a half bias and floor;
prove the admitted arithmetic domain before substituting native rounding, and
keep Lua-specific behavior out of runtime contracts unless its game meaning is
established. No final resource formula or complete build is published here.
