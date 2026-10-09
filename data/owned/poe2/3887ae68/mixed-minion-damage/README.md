# Inherited and applied minion damage

This packet adds a data-defined consumer of the existing inherited damage stat
`1d34` and checked post-stacking application channel `322d`. It publishes
`3353` (their increased-damage subtotal, in percentage points) and `3354`
(`1 + subtotal / 100`, dimensionless). The two ordinary Stat receivers apply to
the existing Sniper Actor slot; each recipient is evaluated independently.
They use the current rule graph and V25 contribution-query capability. There
is no spell, item, build name or source-language dispatch in runtime Rust.

`queries.json` explicitly identifies the reviewed Pain Offering stacking group
and its declaration. It reads the group's result, never the individual source
candidates. `consumer.json` uses typed reads, addition and percentage conversion.
Its only literal is the mathematical identity one. Levels, item bonuses,
selected passive allocations, activation and Offering strength remain upstream
inputs and calculations. The inherited stat retains its existing semantics.

`migration.json` introduces two consecutive definitions and selects V25.
`dependencies.json` authenticates existing source programs, receivers,
application definitions and coverage. Publication adds the two consumer owners,
two receivers and one query through the existing release assembler. It preserves
all existing owners, application/query registry gaps, support obligations and
all five imported builds. The provisional uncommitted incoming-critical draft
previously mentioned `3353`–`3364`; it must allocate again from the current
catalog when adopted. This packet does not edit that draft.

`source-vectors.json` retains eight observed physical-damage increase queries
from the already authenticated PoB witness. Publication compares them to both
byte-identical JIT reports through the existing witness authenticator. They cover
the original and repeated original, disabled and level-1 Offering, duplicate and
unequal Offering copies, and the passive removal control. The latter requires an
explicit native second removal because the saved PoB tree and its active tree
disagree about one disconnected node; no silent tree repair is performed.
These observations are test expectations, never native runtime inputs.

The joined integration executes this data after actual item-driven preparation,
source scaling, application stacking and passive receipt. It also checks missing
inputs, recipient identity, membership and stage failures, storage permutation,
scratch reuse and four-worker Rayon execution.

This is an increased-damage component. It does not establish complete action
damage coverage: conditional/type-specific modifiers, MORE aggregation,
conversion/gain, flat damage, final hit formulas and full-build metrics retain
their separate obligations. No complete build or new global source-domain
closure is claimed. The current endpoint and validation evidence are recorded
in [the implementation plan](../../../../../docs/implementation.md).
