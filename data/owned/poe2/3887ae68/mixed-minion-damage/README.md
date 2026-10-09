# Inherited and applied minion damage factors

This packet adds a data-defined consumer of the existing inherited damage stat
`1d34` and checked post-stacking application channel `322d`. It publishes
`3353` (their increased-damage subtotal, in percentage points) and `3354`
(`1 + subtotal / 100`, dimensionless). It also derives existing `330b` from
projected quality factor `001d` and a checked product of the existing Gigantic
damage contribution. The three ordinary Stat receivers apply to
the existing Sniper Actor slot; each recipient is evaluated independently.
They use the current rule graph and V25 contribution-query capability. There
is no spell, item, build name or source-language dispatch in runtime Rust.

`queries.json` explicitly identifies the reviewed Pain Offering stacking group
and its declaration. It reads the group's result, never the individual source
candidates. `consumer.json` uses typed reads, addition and percentage conversion.
The increased-damage consumer's only literal is the mathematical identity one. Levels, item bonuses,
selected passive allocations, activation and Offering strength remain upstream
inputs and calculations. The inherited stat retains its existing semantics.

The MORE query identifies the existing supplied-Actor program/effect, preserving
its activation and exact recipient. Its complete-empty identity is one;
unknown Gigantic status is unavailable, not an empty contribution. The quality
factor comes from the existing population projection, including prepared quality
and its authored coefficient/truncation; it is not reconstructed from raw gem
quality here. Missing quality does not default to one.

The admitted sources share one reviewed unconditional-damage rounding domain.
The consumer multiplies quality and contributions, scales by 100, adds 0.5,
floors at unit quantum, and divides by 100. These explicit data operations retain
the observed decimal boundary (quality 1 with Gigantic yields 1.21). They do not
encode Lua table ancestry in the native model or impose this rule on all MORE
channels. Different semantic modifier groups and precision policies need their
own reviewed composition. Whether PoB's grouping represents game behavior
remains an explicit item for the Lua-semantics audit.

`migration.json` introduces two consecutive definitions and selects V25.
`dependencies.json` authenticates existing source programs, receivers,
application definitions and coverage. Publication adds three consumer owners,
three receivers and two queries through the existing release assembler. It preserves
all existing owners, application/query registry gaps, support obligations and
all five imported builds. The provisional uncommitted incoming-critical draft
previously mentioned `3353`–`3364`; it must allocate again from the current
catalog when adopted. This packet does not edit that draft.

The current packet is rebuilt from the same checked predecessor; there is no
older-packet compatibility path or second MORE evaluator.

`source-vectors.json` retains twelve observed physical-damage increase/MORE queries
from the already authenticated PoB witness. Publication compares them to both
byte-identical JIT reports through the existing witness authenticator. They cover
the original and repeated original, disabled and level-1 Offering, duplicate and
unequal Offering copies, quality 1/20, Gigantic removal, quality without Gigantic,
and the passive removal control. The latter requires an
explicit native second removal because the saved PoB tree and its active tree
disagree about one disconnected node; no silent tree repair is performed.
These observations are test expectations, never native runtime inputs.

The joined integration executes this data after actual item-driven preparation,
source scaling, application stacking and passive receipt. It also checks missing
inputs, recipient identity, membership and stage failures, storage permutation,
independent quality on repeated recipients, scratch reuse and four-worker Rayon
execution. Source observations normalize the observer's empty table to an array
offline; that representation has no native-runtime meaning.

This is an unconditional increased/MORE component. It does not establish complete action
damage coverage: conditional/type-specific modifiers, further MORE domains,
conversion/gain, flat damage, final hit formulas and full-build metrics retain
their separate obligations. No complete build or new global source-domain
closure is claimed. The current endpoint and validation evidence are recorded
in [the implementation plan](../../../../../docs/implementation.md).
