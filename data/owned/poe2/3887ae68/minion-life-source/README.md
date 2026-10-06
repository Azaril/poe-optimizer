# Intrinsic allied minion Life

This packet injects the complete allied Life table for levels 1–100 and the
reviewed Sniper profile's `0.55` scale. The new Actor program reads the existing
projected Actor level `001c`, looks up the table, scales its value, then floors to
one Life point. It contributes the result to the existing Actor Life/Add channel
`311a`, with Life-points unit `3119`. No final Life scalar or definition ID is
added, and there is no fallback zero or inferred curve interpolation.

Admission is the exact Actor slot `001f`, declared by Skill `0012` and bound to
Actor definition `3091` / `RaisedSkeletonSniper`. The source's complete profile,
original profile/table selection, and original initializer prove its allied
branch. Hostile or replacement profiles are not admitted by this packet; no Lua
absent-or-false behavior or arbitrary hostility input is added to native rules.
The existing `ordinary-population-inputs` program, its `sniper.actor-level`
table and required slots are authenticated unchanged.

The same publication removes a redundant live Life channel: Gigantic's existing
program now writes its `Multiply 1.2` contribution to `311a`, alongside intrinsic
and flat Life `Add` contributions and existing `Increase` contributions. Only
that effect's destination changes. Damage continues to use `330b`. Definition
`330a` remains allocated for historical release identity, with no executable
reader, writer, receiver or alias in the new release.

Publication first uses the existing V5 migration for the genuine table and
program append. It then applies the explicit, exact `replacement.json` through
full endpoint assembly, preserving the append-only migration API. Two inverse
checks prove that only the reviewed owner/provenance changed after rebinding and
that removing the table/append/replacement reconstructs the entire prior recipe.
The Actor owner remains Partial. No queries, routes, receiver inventory,
contributor closure or original build selection is changed.

The canonical witness is
`runs/owned-minion-intrinsic-life-source-02/source-jit-{off,on}.json`, with both
JIT modes producing identical bytes. It observes the original allied-table
selection, actual unrounded initializer local, resulting `Life BASE` record,
and eligibility in the original Life consumer. Nine cases include untouched,
repeated and warm originals, selected CALCS, saved skill levels 1/2/19/20 after
ordinary removal of the two level-granting items, and an explicit authored-domain
level-40 boundary. The last case is not a claim of an obtainable physical gem.
Hooked outputs also match independent unhooked runs. `native_cases` is projected
from those original records, not from a second implementation of the formula.

The default gate authenticates the full 100-row table and exact data contracts;
the opt-in publication gate additionally authenticates full source reports and
preserves all five originals. Native tests exercise the published program,
counterexamples for scaling/floor order and table boundaries, missing inputs,
independent occurrences and reused parallel scratch. Source and native results
are recorded in the [implementation checkpoint](../../../../../docs/implementation.md).
Publication01 passes in 28.37 seconds; all six native tests pass in 23.41 seconds.
The native fixture installs the exact current population facts and requirements,
authenticating both underlying tables. Its explicit final-parent input is a
component boundary; real physical source assembly is checked separately by the
[Sniper input packet](../sniper-final-inputs/README.md).

This is one intrinsic Life contribution. Final Life equations, recipient delivery
of other modifiers, contributor completeness and full-build coverage remain open.
Future curve representations must preserve these exact data and operation-order
semantics; the packet does not fit a curve to observed values.
