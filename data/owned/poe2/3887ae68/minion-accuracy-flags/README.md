# Typed minion accuracy flags

This cutover replaces the integer presence channels `3219` (Player minion
accuracy inheritance) and `321e` (Enemy cannot block attacks) with Boolean
channels at the same identities. The two consumers read resolved Boolean values
from checked contribution queries. No intermediate Stat, definition, receiver,
runtime branch or evaluator operation is added. `321a` remains a singular
derived Boolean for the population's ordinary cannot-be-evaded result.

The query IDs are `minion-accuracy-inheritance-flags` and
`minion-accuracy-cannot-block-flags`. Each has group `sources`, contribution
`Flag`, unordered reduction `Any` and false identity. Their empty member lists
remain **Partial**; they do not prove false or authorize complete native builds.
Unknown membership or values remain unresolved even beside a true contributor.
No actual producer family or support-origin authority is admitted here.

The inherited-player-accuracy branch remains unsupported. A known false
inheritance flag selects the existing ordinary result; known true does not
synthesize an accuracy result. Enemy cannot-block is read from the existing
Action context, so this introduces no shared-Actor-to-Enemy read authority.
Numeric block contribution reads and their source arithmetic stay exact:
`max(min(blockBase, 100) - reduction, 0)`, followed by the known Boolean bypass
and the existing accuracy multiplier. There is no new rounding or final clamp.

`schemas.json`, `programs.json` and `queries.json` are the current authored
replacement data. `dependencies.json` pins the historical programs and retained
source evidence; their integer bodies are used only to authenticate the exact
offline cutover. They are not selectable current runtime behavior. The source
packet projects thirteen Sniper consumers from the existing 31-case witness.
The original hit chance100 and block37 result63 are measured values. Boolean
false/duplicate/unknown controls are native contract tests, not additional source
observations or proof of obtainable game modifiers.

The predecessor is `runs/owned-sniper-activation-readiness-publication-01/package`,
input `7473da9a5220e6e8ddb453cab1caf13f42d8d2f3565d5c0d3d6f27e223defe42`.
The existing offline schema-revision seam rebinds data identities. A checked,
unpublished intermediate temporarily removes exactly the two old consumer
programs; final assembly restores their Boolean replacements at the original
positions and appends both query declarations. One final provenance entry binds
the original input and this packet. The inverse proves exact preservation of
all other schema, rule, route, inventory and import content except the required
dependency commitments. No IDs are allocated and the critical draft's reserved
range is untouched.

The checked publication is
`runs/owned-minion-accuracy-flags-publication-01/package`, input
`949598559007abfc850ee4122616616bfae484f1d30d12b2df26ec2255c23ef3`.
Two ordinary authoring checks pass. Retained-source authentication and all-five
publication pass together in 35.31s, including identical-byte regeneration.
Sixteen tests on the joined item-driven Sniper graph pass in 24.15s, and the five
Sniper preparation plus seven Offering regressions pass. See the
[current checkpoint](../../../../../docs/implementation.md).

The Engine `owned_minion_accuracy_rules` target adds two ordinary CI tests that
compile the current authored programs and replay all thirteen retained controls.
They pass in 0.01s and check missing facts, Boolean typing, inherited-accuracy
refusal and scratch restoration. Supplied `RuleFact` values validate rule laws;
they do not prove provider resolution or complete query membership. The joined
graph tests cover those boundaries but currently require a locally published
package and are ignored by ordinary CI; provisioning that integration subset
remains a tracked delivery task.

Run the `owned_minion_accuracy_flags` target for ordinary authoring checks.
Its ignored publication test requires
`POE_OPTIMIZER_TEST_MINION_ACCURACY_FLAGS_PRIOR` and
`POE_OPTIMIZER_TEST_MINION_ACCURACY_FLAGS_OUTPUT`; it authenticates both retained
JIT reports without executing PoB and preserves all110 original queries and
selected issue counts107/117/109/123/5. Whole-build completion, actual contributor
closure, inherited accuracy and final damage remain outside this checkpoint.
